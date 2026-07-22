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
	Name   string
	State  string
	HasWeb bool // has traefik router label → eligible for quick link
}

type Trace struct {
	Service   string
	Operation string
	Duration  int64
	Spans     int
	HasError  bool
}

type NodeMetrics struct {
	CPU    float64 // percent
	RAM    float64 // percent
	Disk   float64 // percent
	Load1  float64
	Load5  float64
	Load15 float64
	NetIn  float64 // bytes/s
	NetOut float64 // bytes/s
	Online bool
}

type DashboardData struct {
	Services     []Service
	Running      int
	Degraded     int
	TraceCount   int
	RPS          []float64
	Latency      []float64
	Errors       []float64
	TraceVolume  []float64
	TraceList    []Trace
	Labels       []string
	Node         NodeMetrics
	Links        []Link
	HasOTelData  bool
	HasNodeData  bool
	HealthSVG    template.HTML
	RPSSVG       template.HTML
	LatencySVG   template.HTML
	ErrorSVG     template.HTML
	TraceSVG     template.HTML
	CPUSVG       template.HTML
	RAMSVG       template.HTML
	DiskSVG      template.HTML
	SystemName   string
	TotalUp      int
	TotalDown    int
}

type Link struct {
	URL   string
	Label string
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
	node     NodeMetrics
}

var state appState

const maxPts = 30

// ── HTTP clients ──

var dockerClient = &http.Client{
	Transport: &http.Transport{
		Dial: func(_, _ string) (net.Conn, error) {
			return net.DialTimeout("unix", "/var/run/docker.sock", 5*time.Second)
		},
	},
	Timeout: 10 * time.Second,
}

var httpClient = &http.Client{Timeout: 10 * time.Second}

var composeProject string // auto-detected from Docker labels

func fetchJSON(url string, v interface{}) error {
	r, err := httpClient.Get(url)
	if err != nil {
		return err
	}
	defer r.Body.Close()
	return json.NewDecoder(r.Body).Decode(v)
}

// ── Docker service discovery ──

func fetchServices() []Service {
	r, err := dockerClient.Get("http://localhost/containers/json?all=true")
	if err != nil {
		return nil
	}
	defer r.Body.Close()
	var raw []struct {
		Names   []string `json:"Names"`
		State   string   `json:"State"`
		Labels  map[string]string `json:"Labels"`
		NetworkSettings *struct {
			Networks map[string]any `json:"Networks"`
		} `json:"NetworkSettings"`
	}
	if json.NewDecoder(r.Body).Decode(&raw) != nil {
		return nil
	}

	// auto-detect compose project name from first compose-managed container
	if composeProject == "" {
		for _, c := range raw {
			if p, ok := c.Labels["com.docker.compose.project"]; ok && p != "" {
				composeProject = p
				break
			}
		}
	}

	var svcs []Service
	for _, c := range raw {
		if c.NetworkSettings != nil {
			if _, ok := c.NetworkSettings.Networks["app-shared-net"]; ok {
				// only include containers from our compose project
				if c.Labels["com.docker.compose.project"] == composeProject {
					name := strings.TrimPrefix(c.Names[0], "/")
					// auto-detect web services by traefik label
					hasWeb := false
					for k := range c.Labels {
						if strings.HasPrefix(k, "traefik.http.routers.") && strings.HasSuffix(k, ".rule") {
							hasWeb = true
							break
						}
					}
					// also flag containers that look like web services (named with trailing numbers or common patterns)
					// but don't hardcode names — trust only labels
					svcs = append(svcs, Service{Name: name, State: c.State, HasWeb: hasWeb})
				}
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

// ── Prometheus helpers ──

// promInstant returns a single float64 value from an instant query
func promInstant(query string) (float64, bool) {
	u := fmt.Sprintf("http://prometheus:9090/api/v1/query?query=%s",
		url.QueryEscape(query))
	var d struct {
		Data struct {
			Result []struct {
				Value []any `json:"value"`
			} `json:"result"`
		} `json:"data"`
	}
	if fetchJSON(u, &d) != nil || len(d.Data.Result) == 0 || len(d.Data.Result[0].Value) < 2 {
		return 0, false
	}
	var f float64
	fmt.Sscanf(fmt.Sprint(d.Data.Result[0].Value[1]), "%f", &f)
	return f, true
}

// promRange returns time-series values from a range query
func promRange(query string) []float64 {
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

// ── Node metrics ──

func fetchNodeMetrics() NodeMetrics {
	n := NodeMetrics{}

	if v, ok := promInstant(`100 - (avg by(instance)(rate(node_cpu_seconds_total{mode="idle"}[1m])) * 100)`); ok {
		n.CPU = v
	}
	if v, ok := promInstant(`(1 - node_memory_MemAvailable_bytes / node_memory_MemTotal_bytes) * 100`); ok {
		n.RAM = v
	}
	if v, ok := promInstant(`(1 - node_filesystem_avail_bytes{mountpoint="/",fstype!="tmpfs"} / node_filesystem_size_bytes{mountpoint="/",fstype!="tmpfs"}) * 100`); ok {
		n.Disk = v
	}
	if v, ok := promInstant(`node_load1`); ok {
		n.Load1 = v
	}
	if v, ok := promInstant(`node_load5`); ok {
		n.Load5 = v
	}
	if v, ok := promInstant(`node_load15`); ok {
		n.Load15 = v
	}
	if vIn, okIn := promInstant(`rate(node_network_receive_bytes_total{device!="lo"}[1m])`); okIn {
		n.NetIn = vIn
	}
	if vOut, okOut := promInstant(`rate(node_network_transmit_bytes_total{device!="lo"}[1m])`); okOut {
		n.NetOut = vOut
	}

	// node is online if at least CPU or RAM returned data
	n.Online = n.CPU > 0 || n.RAM > 0
	return n
}

// ── SVG Charts ──

func svgDonut(running, degraded int) string {
	total := running + degraded
	if total == 0 {
		return noDataSVG(200, 210)
	}
	const cx, cy, R = 100, 90, 60
	const circ = 2 * math.Pi * R
	type seg struct{ n int; c, l string }
	segs := []seg{{running, "#3fb950", "Running"}, {degraded, "#d29922", "Degraded"}}

	var b strings.Builder
	b.WriteString(`<svg width="200" height="210" viewBox="0 0 200 210" xmlns="http://www.w3.org/2000/svg">`)
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

	b.WriteString(fmt.Sprintf(`<text x="%d" y="%d" text-anchor="middle" fill="#e6edf3" font-size="26" font-weight="700" font-family="system-ui,sans-serif">%d</text>`, cx, cy-4, total))
	b.WriteString(fmt.Sprintf(`<text x="%d" y="%d" text-anchor="middle" class="sl">total</text>`, cx, cy+14))

	y := 165
	for _, s := range segs {
		if s.n == 0 {
			continue
		}
		fmt.Fprintf(&b, `<circle cx="16" cy="%d" r="4" fill="%s"/><text x="26" y="%d" class="sl">%s: %d</text>`, y, s.c, y+3, s.l, s.n)
		y += 16
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
		return noDataSVG(int(w), int(h))
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
	fmt.Fprintf(&b, `<svg width="%.0f" height="%.0f" viewBox="0 0 %.0f %.0f" xmlns="http://www.w3.org/2000/svg">`, w, h, w, h)
	b.WriteString(`<style>.ax{font-family:system-ui,sans-serif;font-size:9px;fill:#6e7681}</style>`)

	for i := 0; i <= 4; i++ {
		y := pt + vh*float64(i)/4
		val := maxV * (1 - float64(i)/4)
		fmt.Fprintf(&b, `<line x1="%.0f" y1="%.0f" x2="%.0f" y2="%.0f" stroke="#21262d" stroke-width="1"/>`, pl, y, pl+vw, y)
		fmt.Fprintf(&b, `<text x="%.0f" y="%.0f" text-anchor="end" class="ax">%.0f</text>`, pl-6, y+3, val)
	}

	n := len(data)
	if n > 1 {
		pts := make([]string, n)
		for i, v := range data {
			x := pl + vw*float64(i)/float64(n-1)
			y := pt + vh*(1-v/maxV)
			pts[i] = fmt.Sprintf("%.1f,%.1f", x, y)
		}
		area := fmt.Sprintf("M%.1f,%.1f L%s L%.1f,%.1f Z", pl, pt+vh, strings.Join(pts, " L"), pl+vw, pt+vh)
		fmt.Fprintf(&b, `<path d="%s" fill="%s" opacity="0.15"/>`, area, color)
		fmt.Fprintf(&b, `<polyline points="%s" fill="none" stroke="%s" stroke-width="2" stroke-linejoin="round"/>`,
			strings.Join(pts, " "), color)

		lv := data[n-1]
		ly := pt + vh*(1-lv/maxV)
		fmt.Fprintf(&b, `<text x="%.0f" y="%.0f" text-anchor="end" fill="%s" font-size="11" font-weight="600" font-family="system-ui,sans-serif">%.1f</text>`,
			pl+vw, ly-10, color, lv)
	}

	step := n / 5
	if step < 1 {
		step = 1
	}
	for i := step; i < n; i += step {
		x := pl + vw*float64(i)/float64(n-1)
		fmt.Fprintf(&b, `<text x="%.0f" y="%.0f" text-anchor="middle" class="ax">%d</text>`, x, pt+vh+16, i+1)
	}
	b.WriteString(`</svg>`)
	return b.String()
}

func svgGauge(pct float64, color, label, unit string) string {
	if pct <= 0 {
		return noDataSVG(220, 100)
	}
	w, h := 220.0, 100.0
	bw, bh := 200.0, 14.0
	bx, by := (w-bw)/2, 30.0
	fw := bw * math.Min(pct/100, 1)

	var b strings.Builder
	fmt.Fprintf(&b, `<svg width="%.0f" height="%.0f" viewBox="0 0 %.0f %.0f" xmlns="http://www.w3.org/2000/svg">`, w, h, w, h)
	b.WriteString(`<style>.gt{font-family:system-ui,sans-serif;font-size:11px;fill:#8b949e;text-anchor:middle}</style>`)

	// background bar
	fmt.Fprintf(&b, `<rect x="%.0f" y="%.0f" width="%.0f" height="%.0f" rx="7" ry="7" fill="#1c2333"/>`, bx, by, bw, bh)
	// fill bar
	if fw > 0 {
		fmt.Fprintf(&b, `<rect x="%.0f" y="%.0f" width="%.0f" height="%.0f" rx="7" ry="7" fill="%s" opacity="0.85"/>`, bx, by, fw, bh, color)
	}
	// label
	fmt.Fprintf(&b, `<text x="%.0f" y="14" class="gt" font-weight="600" fill="%s">%s</text>`, w/2, color, label)
	// value
	fmt.Fprintf(&b, `<text x="%.0f" y="%.0f" class="gt" fill="#e6edf3" font-size="14" font-weight="700">%.1f%s</text>`, w/2, by+bh+24, pct, unit)
	b.WriteString(`</svg>`)
	return b.String()
}

func noDataSVG(w, h int) string {
	return fmt.Sprintf(`<svg width="%d" height="%d" viewBox="0 0 %d %d" xmlns="http://www.w3.org/2000/svg"><text x="%d" y="%d" text-anchor="middle" fill="#6e7681" font-size="12" font-family="system-ui,sans-serif">No data</text></svg>`,
		w, h, w, h, w/2, h/2)
}

// ── Refresh ──

func refresh() {
	svcs := fetchServices()

	// Merge Jaeger-discovered services
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
		o := map[string]int{"running": 0, "jaeger": 1, "exited": 2, "restarting": 3}
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

	// Prometheus OTel metrics
	rps := promRange("rate(otelcol_receiver_accepted_spans[1m])")
	lat := promRange("otelcol_receiver_accepted_spans")
	err := promRange("rate(otelcol_receiver_refused_spans[1m])")

	// Node metrics
	node := fetchNodeMetrics()

	state.mu.Lock()
	state.services = svcs
	state.tracesL = traces
	state.node = node

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

// autoDetectLinks generates links for all services with traefik labels
func autoDetectLinks(svcs []Service) []Link {
	var links []Link
	links = append(links, Link{"/jaeger", "Jaeger UI"})

	hasProm := false
	for _, s := range svcs {
		if s.Name == "prometheus" {
			hasProm = true
		}
	}
	if hasProm {
		links = append(links, Link{"/api/prometheus/targets", "Prometheus"})
	}
	links = append(links, Link{"https://github.com/asepharyana/asepharyana-hub", "GitHub"})

	// auto-generate links for services with traefik labels
	domains := []string{"asepharyana.my.id", "asepharyana.web.id"}
	for _, s := range svcs {
		if s.HasWeb && s.State == "running" {
			for _, d := range domains {
				links = append(links, Link{fmt.Sprintf("https://%s.%s", s.Name, d), s.Name})
				break // one link per service
			}
		}
	}
	return links
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
	node := state.node
	state.mu.Unlock()

	running, degraded := 0, 0
	for _, s := range svcs {
		if s.State == "running" {
			running++
		} else {
			degraded++
		}
	}

	cpuColor, ramColor, diskColor := "#3fb950", "#3fb950", "#3fb950"
	if node.CPU > 80 {
		cpuColor = "#f85149"
	} else if node.CPU > 60 {
		cpuColor = "#d29922"
	}
	if node.RAM > 80 {
		ramColor = "#f85149"
	} else if node.RAM > 60 {
		ramColor = "#d29922"
	}
	if node.Disk > 80 {
		diskColor = "#f85149"
	} else if node.Disk > 60 {
		diskColor = "#d29922"
	}

	data := DashboardData{
		Services: svcs, Running: running, Degraded: degraded,
		TraceCount: len(tr), RPS: rps, Latency: lat, Errors: ers,
		TraceVolume: trc, TraceList: tr, Labels: labels,
		SystemName: composeProject, TotalUp: running, TotalDown: degraded,
		Links:       autoDetectLinks(svcs),
		HasOTelData: len(rps) > 0 || len(lat) > 0, // actual OTel collector metrics
		HasNodeData: node.Online,
		Node:        node,
		HealthSVG:   template.HTML(svgDonut(running, degraded)),
		RPSSVG:      template.HTML(svgLine(rps, "#58a6ff")),
		LatencySVG:  template.HTML(svgLine(lat, "#bc8cff")),
		ErrorSVG:    template.HTML(svgLine(ers, "#f85149")),
		TraceSVG:    template.HTML(svgLine(trc, "#3fb950")),
		CPUSVG:      template.HTML(svgGauge(node.CPU, cpuColor, "CPU Usage", "%")),
		RAMSVG:      template.HTML(svgGauge(node.RAM, ramColor, "Memory Usage", "%")),
		DiskSVG:     template.HTML(svgGauge(node.Disk, diskColor, "Disk Usage", "%")),
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
