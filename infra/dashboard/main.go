package main

import (
	_ "embed"
	"encoding/json"
	"fmt"
	"html/template"
	"io"
	"log"
	"math"
	"net"
	"net/http"
	"net/http/httputil"
	"net/url"
	"os"
	"sort"
	"strings"
	"sync"
	"time"
)

//go:embed template.html
var templateHTML string

var tmpl = template.Must(template.New("dashboard").Funcs(template.FuncMap{
	"sub":       func(a, b int) int { return a - b },
	"divF":     func(a, b float64) float64 { return a / b },
	"hasSuffix": strings.HasSuffix,
	"safeDur": func(us int64) string {
		if us < 1000 {
			return fmt.Sprintf("%dµs", us)
		} else if us < 1_000_000 {
			return fmt.Sprintf("%.1fms", float64(us)/1000)
		}
		return fmt.Sprintf("%.2fs", float64(us)/1_000_000)
	},
}).Parse(templateHTML))

// ── Types ──

type Service struct {
	Name  string
	State string
}

type Trace struct {
	Service   string
	Operation string
	Duration  int64
	Spans     int
	HasError  bool
}

type DashboardData struct {
	Services    []Service
	Running     int
	Down        int
	OTelOnly    int
	TraceCount  int
	RPS         []float64
	Latency     []float64
	Errors      []float64
	Traces      []float64
	Labels      []string
	HealthSVG   template.HTML
	RPSSVG      template.HTML
	LatencySVG  template.HTML
	ErrorSVG    template.HTML
	TraceSVG    template.HTML
	SystemName  string
	Error       string
	TotalUp     int
	TotalDown   int
}

// ── State ──

type appState struct {
	mu       sync.Mutex
	rps      []float64
	latency  []float64
	errs     []float64
	traces   []float64
	labels   []string
	services []Service
	tracesL  []Trace
}

var state appState

const maxPts = 30

// ── Docker socket client ──

var dockerClient = &http.Client{
	Transport: &http.Transport{
		Dial: func(_, _ string) (net.Conn, error) {
			return net.DialTimeout("unix", "/var/run/docker.sock", 5*time.Second)
		},
	},
	Timeout: 10 * time.Second,
}

var httpClient = &http.Client{Timeout: 10 * time.Second}

// ── Helpers ──

func fetchJSON(url string, v interface{}) error {
	r, err := httpClient.Get(url)
	if err != nil {
		return err
	}
	defer r.Body.Close()
	return json.NewDecoder(r.Body).Decode(v)
}

// ── Docker ──

func fetchServices() []Service {
	r, err := dockerClient.Get("http://localhost/containers/json?all=true")
	if err != nil {
		return nil
	}
	defer r.Body.Close()
	var raw []struct {
		Names            []string `json:"Names"`
		State            string   `json:"State"`
		NetworkSettings *struct {
			Networks map[string]any `json:"Networks"`
		} `json:"NetworkSettings"`
	}
	if json.NewDecoder(r.Body).Decode(&raw) != nil {
		return nil
	}
	var svcs []Service
	for _, c := range raw {
		if c.NetworkSettings != nil {
			if _, ok := c.NetworkSettings.Networks["app-shared-net"]; ok {
				svcs = append(svcs, Service{Name: strings.TrimPrefix(c.Names[0], "/"), State: c.State})
			}
		}
	}
	return svcs
}

// ── Jaeger ──

func jaegerServices() []string {
	var d struct{ Data []string }
	if fetchJSON("http://jaeger:16686/api/services", &d) != nil {
		return nil
	}
	return d.Data
}

func jaegerTraces(service string) []Trace {
	now := time.Now().UnixMicro()
	start := now - 5*60*1_000_000
	u := fmt.Sprintf("http://jaeger:16686/api/traces?service=%s&start=%d&end=%d&limit=5&lookback=5m",
		url.QueryEscape(service), start, now)
	var d struct {
		Data []struct {
			Duration  int64 `json:"duration"`
			Spans     []struct {
				OperationName string `json:"operationName"`
				ProcessID     string `json:"processID"`
				Tags          []struct {
					Key   string `json:"key"`
					Value any    `json:"value"`
				} `json:"tags"`
			} `json:"spans"`
			Processes map[string]struct {
				ServiceName string `json:"serviceName"`
			} `json:"processes"`
		} `json:"data"`
	}
	if fetchJSON(u, &d) != nil {
		return nil
	}
	var tt []Trace
	for _, t := range d.Data {
		if len(t.Spans) == 0 {
			continue
		}
		s := t.Spans[0]
		svc := "unknown"
		if p, ok := t.Processes[s.ProcessID]; ok {
			svc = p.ServiceName
		}
		hasErr := false
		for _, sp := range t.Spans {
			for _, tag := range sp.Tags {
				if tag.Key == "error" && tag.Value == true {
					hasErr = true
				}
			}
		}
		tt = append(tt, Trace{Service: svc, Operation: s.OperationName, Duration: t.Duration, Spans: len(t.Spans), HasError: hasErr})
	}
	return tt
}

// ── Prometheus ──

func promQuery(query string) []float64 {
	now := time.Now().Unix()
	u := fmt.Sprintf("http://prometheus:9090/api/v1/query_range?query=%s&start=%d&end=%d&step=15",
		url.QueryEscape(query), now-300, now)
	var d struct {
		Data struct {
			Result []struct {
				Values [][]any `json:"values"`
			} `json:"result"`
		} `json:"data"`
	}
	if fetchJSON(u, &d) != nil || len(d.Data.Result) == 0 {
		return nil
	}
	var vals []float64
	for _, v := range d.Data.Result[0].Values {
		if len(v) == 2 {
			var f float64
			fmt.Sscanf(fmt.Sprint(v[1]), "%f", &f)
			vals = append(vals, f)
		}
	}
	return vals
}

// ── SVG Charts ──

func svgDonut(running, down, otel int) string {
	total := running + down + otel
	if total == 0 {
		return `<svg width="200" height="210" viewBox="0 0 200 210"><text x="100" y="105" text-anchor="middle" fill="#6e7681" font-size="12" font-family="system-ui,sans-serif">No data</text></svg>`
	}
	const cx, cy, R = 100, 90, 60
	const circ = 2 * math.Pi * R
	type seg struct{ n int; c, l string }
	segs := []seg{{running, "#3fb950", "Running"}, {down, "#f85149", "Down"}, {otel, "#58a6ff", "OTel"}}

	var b strings.Builder
	b.WriteString(fmt.Sprintf(`<svg width="200" height="210" viewBox="0 0 200 210" xmlns="http://www.w3.org/2000/svg">`))
	b.WriteString(`<style>.sl{font-family:system-ui,sans-serif;font-size:10px;fill:#8b949e}</style>`)

	var off float64
	for _, s := range segs {
		if s.n == 0 {
			continue
		}
		frac := float64(s.n) / float64(total)
		ln := frac * circ
		b.WriteString(fmt.Sprintf(`<circle cx="%d" cy="%d" r="%d" fill="none" stroke="%s" stroke-width="14" stroke-dasharray="%.1f %.1f" stroke-dashoffset="%.1f" transform="rotate(-90 %d %d)"/>`,
			cx, cy, R, s.c, ln, circ-ln, -off, cx, cy))
		off += ln
	}

	// Center
	b.WriteString(fmt.Sprintf(`<text x="%d" y="%d" text-anchor="middle" fill="#e6edf3" font-size="26" font-weight="700" font-family="system-ui,sans-serif">%d</text>`, cx, cy-4, total))
	b.WriteString(fmt.Sprintf(`<text x="%d" y="%d" text-anchor="middle" class="sl">total</text>`, cx, cy+14))

	// Legend
	y := 165
	for _, s := range segs {
		pct := 0.0
		if total > 0 {
			pct = float64(s.n) / float64(total) * 100
		}
		if s.n > 0 || s.l == "Running" {
			b.WriteString(fmt.Sprintf(`<circle cx="16" cy="%d" r="4" fill="%s"/>`, y, s.c))
			b.WriteString(fmt.Sprintf(`<text x="26" y="%d" class="sl">%s: %d (%.0f%%)</text>`, y+3, s.l, s.n, pct))
			y += 16
		}
	}
	b.WriteString(`</svg>`)
	return b.String()
}

func svgLine(data []float64, color string) string {
	w, h := 300.0, 160.0
	pl, pt, pr, pb := 45.0, 20.0, 10.0, 25.0
	vw := w - pl - pr
	vh := h - pt - pb

	if len(data) == 0 {
		return fmt.Sprintf(`<svg width="%.0f" height="%.0f" viewBox="0 0 %.0f %.0f" xmlns="http://www.w3.org/2000/svg">
			<text x="%.0f" y="%.0f" text-anchor="middle" fill="#6e7681" font-size="11" font-family="system-ui,sans-serif">No data</text></svg>`,
			w, h, w, h, w/2, h/2)
	}

	maxV := 0.0
	for _, v := range data {
		if v > maxV {
			maxV = v
		}
	}
	if maxV <= 0 {
		maxV = 1
	}

	var b strings.Builder
	b.WriteString(fmt.Sprintf(`<svg width="%.0f" height="%.0f" viewBox="0 0 %.0f %.0f" xmlns="http://www.w3.org/2000/svg">`, w, h, w, h))
	b.WriteString(`<style>.ax{font-family:system-ui,sans-serif;font-size:9px;fill:#6e7681}</style>`)

	// Grid
	for i := 0; i <= 4; i++ {
		y := pt + vh*float64(i)/4
		val := maxV * (1 - float64(i)/4)
		b.WriteString(fmt.Sprintf(`<line x1="%.0f" y1="%.0f" x2="%.0f" y2="%.0f" stroke="#21262d" stroke-width="1"/>`, pl, y, pl+vw, y))
		b.WriteString(fmt.Sprintf(`<text x="%.0f" y="%.0f" text-anchor="end" class="ax">%.0f</text>`, pl-6, y+3, val))
	}

	n := len(data)
	if n > 1 {
		pts := make([]string, n)
		for i, v := range data {
			x := pl + vw*float64(i)/float64(n-1)
			y := pt + vh*(1-v/maxV)
			pts[i] = fmt.Sprintf("%.1f,%.1f", x, y)
		}

		// Area
		area := fmt.Sprintf("M%.1f,%.1f L%s L%.1f,%.1f Z", pl, pt+vh, strings.Join(pts, " L"), pl+vw, pt+vh)
		b.WriteString(fmt.Sprintf(`<path d="%s" fill="%s" opacity="0.15"/>`, area, color))

		// Line
		b.WriteString(fmt.Sprintf(`<polyline points="%s" fill="none" stroke="%s" stroke-width="2" stroke-linejoin="round"/>`,
			strings.Join(pts, " "), color))

		// Last value
		lv := data[n-1]
		ly := pt + vh*(1-lv/maxV)
		b.WriteString(fmt.Sprintf(`<text x="%.0f" y="%.0f" text-anchor="end" fill="%s" font-size="11" font-weight="600" font-family="system-ui,sans-serif">%.1f</text>`,
			pl+vw, ly-10, color, lv))
	}

	// X labels
	step := n / 5
	if step < 1 {
		step = 1
	}
	for i := step; i < n; i += step {
		x := pl + vw*float64(i)/float64(n-1)
		b.WriteString(fmt.Sprintf(`<text x="%.0f" y="%.0f" text-anchor="middle" class="ax">%d</text>`, x, pt+vh+16, i+1))
	}

	b.WriteString(`</svg>`)
	return b.String()
}

// ── Refresh ──

func refresh() {
	svcs := fetchServices()

	jaegerS := jaegerServices()
	for _, s := range jaegerS {
		found := false
		for _, sv := range svcs {
			if sv.Name == s {
				found = true
				break
			}
		}
		if !found {
			svcs = append(svcs, Service{Name: s, State: "jaeger"})
		}
	}
	sort.Slice(svcs, func(i, j int) bool {
		o := map[string]int{"running": 0, "jaeger": 1, "paused": 2, "exited": 3, "restarting": 4}
		oi, oj := o[svcs[i].State], o[svcs[j].State]
		if oi != oj {
			return oi < oj
		}
		return svcs[i].Name < svcs[j].Name
	})

	var traces []Trace
	for _, s := range svcs {
		if s.State == "running" {
			t := jaegerTraces(s.Name)
			traces = append(traces, t...)
		}
	}
	sort.Slice(traces, func(i, j int) bool { return traces[i].Duration > traces[j].Duration })
	if len(traces) > 20 {
		traces = traces[:20]
	}

	// Prometheus
	rps := promQuery("rate(otelcol_receiver_accepted_spans[1m])")
	lat := promQuery("otelcol_receiver_accepted_spans")
	err := promQuery("rate(otelcol_receiver_refused_spans[1m])")

	state.mu.Lock()
	state.services = svcs
	state.tracesL = traces

	now := time.Now().Format("15:04:05")
	state.labels = append(state.labels, now)
	if len(state.labels) > maxPts {
		state.labels = state.labels[len(state.labels)-maxPts:]
	}

	add := func(dst *[]float64, src []float64) {
		v := 0.0
		if len(src) > 0 {
			v = src[len(src)-1]
		}
		*dst = append(*dst, v)
		if len(*dst) > maxPts {
			*dst = (*dst)[len(*dst)-maxPts:]
		}
	}
	add(&state.rps, rps)
	add(&state.latency, lat)
	add(&state.errs, err)
	state.traces = append(state.traces, float64(len(traces)))
	if len(state.traces) > maxPts {
		state.traces = state.traces[len(state.traces)-maxPts:]
	}
	state.mu.Unlock()
}

// ── Dashboard Handler ──

func dashboard(w http.ResponseWriter, _ *http.Request) {
	state.mu.Lock()
	svcs := append([]Service{}, state.services...)
	tr := append([]Trace{}, state.tracesL...)
	rps := append([]float64{}, state.rps...)
	lat := append([]float64{}, state.latency...)
	ers := append([]float64{}, state.errs...)
	trc := append([]float64{}, state.traces...)
	labels := append([]string{}, state.labels...)
	state.mu.Unlock()

	running, down, otel := 0, 0, 0
	for _, s := range svcs {
		switch s.State {
		case "running":
			running++
		case "jaeger":
			otel++
		default:
			down++
		}
	}

	maxLat := 0.0
	for _, v := range lat {
		if v > maxLat {
			maxLat = v
		}
	}

	data := DashboardData{
		Services: svcs, Running: running, Down: down, OTelOnly: otel,
		TraceCount: len(tr), RPS: rps, Latency: lat, Errors: ers, Traces: trc, Labels: labels,
		SystemName: "asepharyana-hub", TotalUp: running, TotalDown: down + otel,
		HealthSVG:  template.HTML(svgDonut(running, down, otel)),
		RPSSVG:     template.HTML(svgLine(rps, "#58a6ff")),
		LatencySVG: template.HTML(svgLine(lat, "#bc8cff")),
		ErrorSVG:   template.HTML(svgLine(ers, "#f85149")),
		TraceSVG:   template.HTML(svgLine(trc, "#3fb950")),
	}

	w.Header().Set("Content-Type", "text/html; charset=utf-8")
	if err := tmpl.Execute(w, data); err != nil {
		http.Error(w, err.Error(), 500)
	}
}

// ── Proxy ──

func proxy(target string) http.Handler {
	u, _ := url.Parse(target)
	return httputil.NewSingleHostReverseProxy(u)
}

func dockerHandler(w http.ResponseWriter, r *http.Request) {
	switch r.URL.Path {
	case "/api/docker/containers/json", "/api/docker/version":
		r.URL.Path = strings.TrimPrefix(r.URL.Path, "/api/docker")
		resp, err := dockerClient.Get("http://localhost" + r.URL.String() + "?" + r.URL.RawQuery)
		if err != nil {
			http.Error(w, err.Error(), 502)
			return
		}
		defer resp.Body.Close()
		for k, v := range resp.Header {
			w.Header()[k] = v
		}
		w.WriteHeader(resp.StatusCode)
		io.Copy(w, resp.Body)
	default:
		http.Error(w, "Forbidden", 403)
	}
}

func healthHandler(w http.ResponseWriter, _ *http.Request) {
	resp, err := http.Get("http://otel-collector:13133/")
	if err != nil {
		http.Error(w, err.Error(), 502)
		return
	}
	defer resp.Body.Close()
	io.Copy(w, resp.Body)
}

// ── Main ──

func main() {
	port := os.Getenv("PORT")
	if port == "" {
		port = "8080"
	}

	refresh()
	go func() {
		for range time.Tick(15 * time.Second) {
			refresh()
		}
	}()

	mux := http.NewServeMux()
	mux.HandleFunc("/", dashboard)
	mux.Handle("/api/jaeger/", http.StripPrefix("/api/jaeger", proxy("http://jaeger:16686/")))
	mux.Handle("/api/prometheus/", http.StripPrefix("/api/prometheus", proxy("http://prometheus:9090/")))
	mux.HandleFunc("/api/health", healthHandler)
	mux.HandleFunc("/api/docker/", dockerHandler)
	mux.Handle("/jaeger/", http.StripPrefix("/jaeger", proxy("http://jaeger:16686/")))

	log.Printf("Dashboard listening on :%s", port)
	log.Fatal(http.ListenAndServe(":"+port, mux))
}
