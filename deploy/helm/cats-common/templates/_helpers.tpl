{{/*
公共 helper 模板，被所有 cats-* service chart 引用。
引用: Helm 官方最佳实践 + ArgoCD 标签约定。
*/}}

{{/*
展开 chart 名: cats-<service>
*/}}
{{- define "cats-common.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" -}}
{{- end -}}

{{/*
展开 fullname: <release>-<chart>
*/}}
{{- define "cats-common.fullname" -}}
{{- if .Values.fullnameOverride -}}
{{- .Values.fullnameOverride | trunc 63 | trimSuffix "-" -}}
{{- else -}}
{{- $name := default .Chart.Name .Values.nameOverride -}}
{{- if contains $name .Release.Name -}}
{{- .Release.Name | trunc 63 | trimSuffix "-" -}}
{{- else -}}
{{- printf "%s-%s" .Release.Name $name | trunc 63 | trimSuffix "-" -}}
{{- end -}}
{{- end -}}
{{- end -}}

{{/*
Chart 名 + 版本标签
*/}}
{{- define "cats-common.chart" -}}
{{- printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" | trunc 63 | trimSuffix "-" -}}
{{- end -}}

{{/*
通用 labels（per K8s recommended labels）
*/}}
{{- define "cats-common.labels" -}}
helm.sh/chart: {{ include "cats-common.chart" . }}
{{ include "cats-common.selectorLabels" . }}
{{- if .Chart.AppVersion }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
{{- end }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
app.kubernetes.io/part-of: cats
{{- end -}}

{{/*
Selector labels（用于 Deployment.spec.selector.matchLabels 与 Pod template）
*/}}
{{- define "cats-common.selectorLabels" -}}
app.kubernetes.io/name: {{ include "cats-common.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end -}}

{{/*
ServiceAccount 名
*/}}
{{- define "cats-common.serviceAccountName" -}}
{{- if .Values.serviceAccount.create -}}
{{- default (include "cats-common.fullname" .) .Values.serviceAccount.name -}}
{{- else -}}
{{- default "default" .Values.serviceAccount.name -}}
{{- end -}}
{{- end -}}
