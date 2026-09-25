#!/usr/bin/env bash
# vercelserver/scripts/build.sh
#
# 用途: 从 SOURCE_ROOT (默认 .. = 父目录 = 主项目) 拉取源, 构建 cats-client,
#       复制 web-console-mock + media-pipeline-mock 到 vercelserver/src/.
#       复用方式: 在另一个项目里复制整个 vercelserver/, 设 SOURCE_ROOT=/path/to/project 即可.
#
# 注意: 必须在 vercelserver/ 目录跑 (或显式 cd). bash (MSYS / git-bash) 兼容.

set -euo pipefail

# 1. 定位
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"   # vercelserver/
SOURCE_ROOT="${SOURCE_ROOT:-$(cd "${ROOT_DIR}/.." && pwd)}"  # 默认父目录

echo "[build.sh] vercelserver dir:  ${ROOT_DIR}"
echo "[build.sh] source root:       ${SOURCE_ROOT}"

# 2. 复制 web-console-mock (静态 HTML, 不需构建)
if [[ -d "${SOURCE_ROOT}/deploy/web-console-mock" ]]; then
  echo "[build.sh] copy web-console-mock/"
  rm -rf "${ROOT_DIR}/src/web-console-mock"
  cp -r "${SOURCE_ROOT}/deploy/web-console-mock" "${ROOT_DIR}/src/web-console-mock"
else
  echo "[build.sh] WARN: ${SOURCE_ROOT}/deploy/web-console-mock not found"
fi

# 3. 复制 media-pipeline-mock (静态 HTML, 不需构建)
if [[ -d "${SOURCE_ROOT}/vercelserver/src/media-pipeline-mock" ]]; then
  echo "[build.sh] copy media-pipeline-mock/ (from vercelserver)"
  rm -rf "${ROOT_DIR}/src/media-pipeline-mock"
  cp -r "${SOURCE_ROOT}/vercelserver/src/media-pipeline-mock" "${ROOT_DIR}/src/media-pipeline-mock"
fi

# 4. 构建 cats-client (Svelte 5 + Vite)
if [[ -d "${SOURCE_ROOT}/apps/cats-client" ]]; then
  echo "[build.sh] build cats-client (npm install + vite build)"
  pushd "${SOURCE_ROOT}/apps/cats-client" >/dev/null

  # 兼容 corp proxy 环境 (per CATs ULYS-110 memory)
  export HTTPS_PROXY="${HTTPS_PROXY:-http://127.0.0.1:10808}"
  export HTTP_PROXY="${HTTP_PROXY:-http://127.0.0.1:10808}"

  npm install --no-audit --no-fund --prefer-offline --no-progress \
    --proxy "${HTTPS_PROXY}" --https-proxy "${HTTPS_PROXY}" --strict-ssl false

  npm run build

  popd >/dev/null

  echo "[build.sh] copy cats-client/dist → vercelserver/src/cats-client"
  rm -rf "${ROOT_DIR}/src/cats-client"
  cp -r "${SOURCE_ROOT}/apps/cats-client/dist" "${ROOT_DIR}/src/cats-client"
else
  echo "[build.sh] WARN: ${SOURCE_ROOT}/apps/cats-client not found"
fi

# 5. 落地 landing page (index.html + styles.css)
#    这两个文件随本目录 commit, 不依赖 SOURCE_ROOT, 仅 sanity check.
for f in index.html styles.css; do
  if [[ ! -f "${ROOT_DIR}/src/${f}" ]]; then
    echo "[build.sh] ERROR: ${ROOT_DIR}/src/${f} missing (landing page broken)"
    exit 1
  fi
done

echo "[build.sh] done. output at ${ROOT_DIR}/src/"
ls -la "${ROOT_DIR}/src/"
