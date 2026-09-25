#!/usr/bin/env python3
"""cargo build --release 12 service, log to file. PowerShell safe."""
import subprocess
import sys
import time

PKGS = [
    "cats-common",
    "cats-rbac",
    "auth-service",
    "user-service",
    "project-service",
    "task-service",
    "file-service",
    "notification-service",
    "report-service",
    "audit-service",
    "worker-service",
    "translation-core",
    "cats-ai-gateway",
    "cats-bff",
]

LOG = "D:/CATs/deploy/cargo-build.log"

def main():
    print(f"==> starting cargo build --release ({len(PKGS)} packages)", flush=True)
    print(f"    log: {LOG}", flush=True)

    cmd = ["cargo", "build", "--release", "-j", "2"]
    for p in PKGS:
        cmd += ["-p", p]

    with open(LOG, "w", encoding="utf-8") as f:
        f.write(f"# cargo build started {time.strftime('%Y-%m-%d %H:%M:%S')}\n")
        f.write(f"# cmd: {' '.join(cmd)}\n\n")
        f.flush()

        proc = subprocess.Popen(
            cmd,
            cwd="D:/CATs",
            stdout=f,
            stderr=subprocess.STDOUT,
            text=True,
        )

        # Poll periodically to report progress
        last_size = 0
        while proc.poll() is None:
            time.sleep(30)
            try:
                size = f.tell()
                if size > last_size + 10000:
                    print(f"    build log size: {size} bytes (running...)", flush=True)
                    last_size = size
            except Exception:
                pass

        rc = proc.returncode
        print(f"==> cargo build finished rc={rc}", flush=True)
        print(f"    see {LOG} for full output", flush=True)
        sys.exit(rc)

if __name__ == "__main__":
    main()