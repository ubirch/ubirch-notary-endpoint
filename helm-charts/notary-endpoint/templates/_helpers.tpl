{{/* vim: set filetype=mustache: */}}

{{/*
    Create a default fully qualified app name.
    We truncate at 63 chars because some Kubernetes name fields are limited to this (by the DNS naming spec).
*/}}
{{- define "fullname" -}}
    {{- $name := default .Chart.Name -}}
    {{- printf "%s-%s" .Release.Name $name | trunc 63 | trimSuffix "-" -}}
{{- end -}}

{{- define "chartName" -}}
    {{- printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" | quote -}}
{{- end -}}
