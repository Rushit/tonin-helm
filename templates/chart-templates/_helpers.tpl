{{/*
Expand the name of the release (used as the k8s resource name).
*/}}
{{- define "service.name" -}}
{{- .Release.Name }}
{{- end }}

{{/*
Common labels applied to every resource.
*/}}
{{- define "service.labels" -}}
app: {{ include "service.name" . }}
app.kubernetes.io/name: {{ include "service.name" . }}
app.kubernetes.io/version: {{ .Values.image.tag | quote }}
helm.sh/chart: {{ .Chart.Name }}-{{ .Chart.Version }}
{{- end }}

{{/*
Selector labels (stable subset — must not change after first deploy).
*/}}
{{- define "service.selectorLabels" -}}
app: {{ include "service.name" . }}
{{- end }}
