#!/usr/bin/env bash
# local-down.sh - 一键本地停
# 用法: bash crates/cats-mock/scripts/local-down.sh

set -euo pipefail

echo "==> stopping cats apps"
kubectl delete -k deploy/k3s/cats-core/ --ignore-not-found
kubectl delete -f deploy/k3s/cats-edge/envoy-deployment.yaml --ignore-not-found

echo "==> deleting k3d cluster"
k3d cluster delete cats-mvp

echo "==> MVP DOWN"