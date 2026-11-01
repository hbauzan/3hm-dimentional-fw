#!/usr/bin/env bash
# Tracer bullet 2: native MPS lifecycle in setup.sh. Mocks uv/docker/curl.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
# shellcheck disable=SC1091
source "$ROOT/setup.sh"

fail() {
    echo "FAIL: $*" >&2
    exit 1
}

if ! declare -p PID_FILE_TEI_MPS >/dev/null 2>&1; then
    fail "PID_FILE_TEI_MPS is unbound"
fi
if ! declare -p LOG_TEI_MPS >/dev/null 2>&1; then
    fail "LOG_TEI_MPS is unbound"
fi
[ "$PID_FILE_TEI_MPS" = ".sf_tei_mps.pid" ] || fail "pid filename"
[ "$LOG_TEI_MPS" = "sf_tei_mps.log" ] || fail "log filename"
declare -F use_native_mps_embedder >/dev/null || fail "missing use_native_mps_embedder"
declare -F tei_native_running >/dev/null || fail "missing tei_native_running"
declare -F stop_native_mps_process >/dev/null || fail "missing stop_native_mps_process"

make_bin() {
    local workdir="$1"
    local bin="$workdir/bin"
    mkdir -p "$bin"
    cat >"$bin/docker" <<EOF
#!/bin/bash
printf '%s\n' "\$*" >> "$workdir/docker.log"
exit 0
EOF
    cat >"$bin/uv" <<EOF
#!/bin/bash
printf '%s\n' "\$*" >> "$workdir/uv.log"
exec sleep 30
EOF
    cat >"$bin/curl" <<EOF
#!/bin/bash
if [[ "\$*" == *"/health"* ]]; then
    printf '200'
    exit 0
fi
printf '000'
exit 0
EOF
    chmod +x "$bin/docker" "$bin/uv" "$bin/curl"
}

assert_stop_kills_pid_and_compose_down() {
    local workdir
    workdir="$(mktemp -d)"
    make_bin "$workdir"
    sleep 120 &
    local sleep_pid=$!
    echo "$sleep_pid" >"$workdir/.sf_tei_mps.pid"
    (
        cd "$workdir"
        export PATH="$workdir/bin:$PATH"
        stop_tei_daemon >/dev/null
    )
    if kill -0 "$sleep_pid" 2>/dev/null; then
        kill -9 "$sleep_pid" 2>/dev/null || true
        rm -rf "$workdir"
        fail "stop_tei_daemon left native pid alive"
    fi
    [ ! -f "$workdir/.sf_tei_mps.pid" ] || {
        rm -rf "$workdir"
        fail "pid file survived stop_tei_daemon"
    }
    grep -q "compose -f docker-compose.tei.yml down" "$workdir/docker.log" || {
        rm -rf "$workdir"
        fail "stop_tei_daemon did not compose down"
    }
    rm -rf "$workdir"
    echo "PASS: stop_tei_daemon SIGTERM + compose down"
}

assert_darwin_start_uses_uv_not_docker_up() {
    local workdir
    workdir="$(mktemp -d)"
    make_bin "$workdir"
    (
        cd "$workdir"
        export PATH="$workdir/bin:$PATH"
        use_native_mps_embedder() { return 0; }
        start_tei_daemon
    )
    grep -q "run tools/bge_m3_mps_server.py" "$workdir/uv.log" || {
        stop_native_mps_process >/dev/null 2>&1 || true
        rm -rf "$workdir"
        fail "start_tei_daemon did not launch uv run tools/bge_m3_mps_server.py"
    }
    if [ -f "$workdir/docker.log" ] && grep -q "up -d" "$workdir/docker.log"; then
        stop_native_mps_process >/dev/null 2>&1 || true
        rm -rf "$workdir"
        fail "native start issued docker compose up"
    fi
    [ -s "$workdir/.sf_tei_mps.pid" ] || {
        rm -rf "$workdir"
        fail "native start did not write .sf_tei_mps.pid"
    }
    (
        cd "$workdir"
        export PATH="$workdir/bin:$PATH"
        stop_tei_daemon >/dev/null
    )
    rm -rf "$workdir"
    echo "PASS: Darwin start uses uv and writes the pid file"
}

assert_docker_path_still_composes_up() {
    local workdir
    workdir="$(mktemp -d)"
    make_bin "$workdir"
    (
        cd "$workdir"
        export PATH="$workdir/bin:$PATH"
        use_native_mps_embedder() { return 1; }
        start_tei_daemon
    )
    grep -q "compose -f docker-compose.tei.yml up -d" "$workdir/docker.log" || {
        rm -rf "$workdir"
        fail "non-mps host did not docker compose up"
    }
    rm -rf "$workdir"
    echo "PASS: non-mps host still uses docker compose up"
}

assert_probe_native_healthy_and_down() {
    local workdir
    workdir="$(mktemp -d)"
    make_bin "$workdir"
    sleep 120 &
    local sleep_pid=$!
    echo "$sleep_pid" >"$workdir/.sf_tei_mps.pid"
    local state
    state="$(
        cd "$workdir"
        export PATH="$workdir/bin:$PATH"
        port_listening() { return 0; }
        probe_tei
    )"
    [ "$state" = "healthy" ] || {
        kill -9 "$sleep_pid" 2>/dev/null || true
        rm -rf "$workdir"
        fail "probe_tei native expected healthy, got $state"
    }
    kill -9 "$sleep_pid" 2>/dev/null || true
    wait "$sleep_pid" 2>/dev/null || true
    rm -f "$workdir/.sf_tei_mps.pid"
    state="$(
        cd "$workdir"
        export PATH="$workdir/bin:$PATH"
        port_listening() { return 1; }
        tei_health_ok() { return 1; }
        probe_tei
    )"
    rm -rf "$workdir"
    [ "$state" = "down" ] || fail "probe_tei expected down, got $state"
    echo "PASS: probe_tei healthy via pid and down when idle"
}

assert_stop_kills_pid_and_compose_down
assert_darwin_start_uses_uv_not_docker_up
assert_docker_path_still_composes_up
assert_probe_native_healthy_and_down
echo "ALL_SETUP_TEI_MPS_TESTS_GREEN"
