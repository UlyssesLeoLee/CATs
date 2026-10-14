#!/usr/bin/env bash
# local-up.sh - 一键本地起 (per deploy/MVP_RUNBOOK.md)
# 用法: bash crates/cats-mock/scripts/local-up.sh

set -euo pipefail

echo "==> [1/4] starting k3d cluster"
k3d cluster create cats-mvp --servers 1 --agents 2 --port "8080:80@loadbalancer"

echo "==> [2/4] deploying cats-edge + cats-core + cats-data"
kubectl apply -f deploy/k3s/namespaces.yaml
kubectl apply -f deploy/k3s/postgres/
kubectl apply -f deploy/k3s/kafka/cats-kafka-mvp.yaml
kubectl apply -f deploy/k3s/kafka/cats-kafka-topics-mvp.yaml
kubectl apply -k deploy/k3s/cats-core/
kubectl apply -f deploy/k3s/cats-edge/envoy-deployment.yaml

echo "==> [3/4] waiting for pods ready"
kubectl wait --for=condition=Ready pods --all -A -l 'app.kubernetes.io/part-of=cats' --timeout=120s

echo "==> [4/4] smoke test"
bash crates/cats-mock/scripts/smoke-test.sh

echo "==> MVP UP"