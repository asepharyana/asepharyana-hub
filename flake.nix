{
  description = "Asepharyana Hub — Nix builds for infrastructure and app services";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachSystem [ "x86_64-linux" ] (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          config.allowUnfree = true;
        };

        # ── mkApp generator ──
        mkApp = { name, src, buildScript, installScript, nativeBuildInputs ? [], buildInputs ? [] }:
          pkgs.stdenv.mkDerivation {
            inherit name src;

            nativeBuildInputs = with pkgs; [
              cacert curl gcc gnumake openssl pkg-config python3 libclang
            ] ++ nativeBuildInputs;

            buildInputs = with pkgs; [
              nodejs openssl stdenv.cc.cc.lib libffi
            ] ++ buildInputs;

            LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";
            LD_LIBRARY_PATH = "${pkgs.libclang.lib}/lib:${pkgs.stdenv.cc.cc.lib}/lib:${pkgs.libffi}/lib";
            NIX_ENFORCE_PURITY = "0";

            SSL_CERT_FILE = "${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt";
            NODE_EXTRA_CA_CERTS = "${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt";
            NODE_ENV = "production";

            phases = [ "unpackPhase" "buildPhase" "installPhase" ];
            buildPhase = ''
              export HOME="$TMPDIR" CARGO_HOME="$TMPDIR/.cargo-${name}"
              SSL_CERT_FILE=${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt
            '' + buildScript;
            installPhase = installScript;
          };

        # ── Node.js ──
        nodejs = pkgs.nodejs-slim_22;
        pnpm = pkgs.pnpm.override { inherit nodejs; };

        # ── Common Rust build deps ──
        cargoDeps = with pkgs; [ rustc cargo clang cmake pkg-config openssl.dev zlib ];

        # ── Submodule repos — URLs from .gitmodules ──
        submoduleRepos = {
          hub = "https://github.com/asepharyana/asepharyana-hub-hub.git";
          scraper = "https://github.com/asepharyana/asepharyana-hub-scraper.git";
          tools = "https://github.com/asepharyana/asepharyana-hub-tools.git";
          llm-api = "https://github.com/asepharyana/asepharyana-hub-llm-api.git";
        };

        # ── Fetch submodule source ──
        submoduleSrc = name: builtins.fetchGit {
          url = submoduleRepos.${name};
          rev = if name == "hub" then "6829c7efe2a735d249f5135bff4a0fc28411a295"
               else if name == "scraper" then "62aa5b0e52859afe3ba9de1c7b11cfe2dacf6c2c"
               else if name == "tools" then "3956b90c3ce39ffa7ffba8084937f20e11364d6b"
               else if name == "llm-api" then "67861f384bd4f64d5236a9608b33c965fbebbc7f"
               else "HEAD";
          submodules = true;
        };

        # ─── App Derivations ───
        hub = mkApp {
          name = "hub-0.1.0";
          src = submoduleSrc "hub";

          nativeBuildInputs = with pkgs; [ bun ];

          buildScript = ''
            echo "=== Installing dependencies ==="
            bun install 2>&1
            echo "=== Building Next.js ==="
            bun run build 2>&1
          '';

          installScript = ''
            mkdir -p $out/share/hub $out/bin
            cp -r .next $out/share/hub/
            cp -r public $out/share/hub/ 2>/dev/null || true
            cp package.json $out/share/hub/
            cp next.config.{ts,mjs,js} $out/share/hub/ 2>/dev/null || true
            cp -r node_modules $out/share/hub/
            cat > $out/bin/hub << WRAPPER
#!${pkgs.runtimeShell}
exec ${pkgs.bun}/bin/bun run --cwd $out/share/hub start
WRAPPER
            chmod +x $out/bin/hub
          '';
        };

        scraper = mkApp {
          name = "scraper-0.1.0";
          src = submoduleSrc "scraper";
          nativeBuildInputs = cargoDeps;

          buildScript = ''
            echo "=== Building scraper ==="
            cargo build --release 2>&1
          '';

          installScript = ''
            mkdir -p $out/bin
            cp target/release/scraper $out/bin/scraper
          '';
        };

        tools-gateway = mkApp {
          name = "tools-gateway-0.1.0";
          src = submoduleSrc "tools";
          nativeBuildInputs = cargoDeps ++ [ pkgs.tesseract ];

          buildScript = ''
            cd backend
            echo "=== Building tools-gateway ==="
            cargo build --release --features tesseract --bin tools-gateway 2>&1
          '';

          installScript = ''
            mkdir -p $out/bin
            cp target/release/tools-gateway $out/bin/tools-gateway
          '';
        };

        tools-workers = mkApp {
          name = "tools-workers-0.1.0";
          src = submoduleSrc "tools";
          nativeBuildInputs = cargoDeps ++ [ pkgs.tesseract pkgs.leptonica ];

          buildScript = ''
            cd backend
            echo "=== Building tools-workers ==="
            cargo build --release --features tesseract --bin tools-workers 2>&1
          '';
          installScript = ''
            mkdir -p $out/bin
            cp target/release/tools-workers $out/bin/tools-workers
          '';
        };

        tools-frontend = mkApp {
          name = "tools-frontend-0.1.0";
          src = submoduleSrc "tools";
          nativeBuildInputs = with pkgs; [ bun ];

          buildScript = ''
            cd frontend
            echo "=== Installing dependencies ==="
            bun install 2>&1
            echo "=== Building Next.js ==="
            bun run build 2>&1
          '';

          installScript = ''
            mkdir -p $out/share/tools-frontend $out/bin
            cp -r .next $out/share/tools-frontend/
            cp -r public $out/share/tools-frontend/ 2>/dev/null || true
            cp package.json $out/share/tools-frontend/
            cp -r node_modules $out/share/tools-frontend/
            cat > $out/bin/tools-frontend << WRAPPER
#!${pkgs.runtimeShell}
exec ${pkgs.bun}/bin/bun run --cwd $out/share/tools-frontend start
WRAPPER
            chmod +x $out/bin/tools-frontend
          '';
        };

        llm-api = mkApp {
          name = "llm-api-0.1.0";
          src = submoduleSrc "llm-api";
          nativeBuildInputs = cargoDeps ++ [ pkgs.cmake pkgs.gcc ];

          buildScript = ''
            echo "=== Building llm-api ==="
            cargo build --release 2>&1
          '';

          installScript = ''
            mkdir -p $out/bin
            cp target/release/llm-api $out/bin/llm-api
          '';
        };

      in
      {
        packages = {
          inherit hub scraper tools-gateway tools-workers tools-frontend llm-api;
          default = hub;
        };

        apps.hub = {
          type = "app";
          program = "${hub}/bin/hub";
        };

        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [ nodejs-slim_22 bun pnpm rustc cargo ];
        };
      });
}
