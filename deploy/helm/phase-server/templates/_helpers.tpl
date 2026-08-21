{{- define "phase-server.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" -}}
{{- end -}}

{{- define "phase-server.fullname" -}}
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

{{- define "phase-server.labels" -}}
helm.sh/chart: {{ printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" }}
{{ include "phase-server.selectorLabels" . }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
{{- end -}}

{{- define "phase-server.selectorLabels" -}}
app.kubernetes.io/name: {{ include "phase-server.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end -}}

{{- define "phase-server.image" -}}
{{- $tag := default (printf "v%s" .Chart.AppVersion) .Values.image.tag -}}
{{- if .Values.image.digest -}}
{{- printf "%s:%s@%s" .Values.image.repository $tag .Values.image.digest -}}
{{- else -}}
{{- printf "%s:%s" .Values.image.repository $tag -}}
{{- end -}}
{{- end -}}

{{- define "phase-server.publicUrl" -}}
{{- default (printf "https://%s" .Values.ingress.host) .Values.server.publicUrl -}}
{{- end -}}

{{- define "phase-server.tlsSecretName" -}}
{{- default (printf "%s-tls" (include "phase-server.fullname" .)) .Values.ingress.tls.secretName -}}
{{- end -}}

{{/* "<ns>-<fullname>-<suffix>@kubernetescrd" — Traefik's name for a Middleware/TLSOption CRD */}}
{{- define "phase-server.crdRef" -}}
{{- printf "%s-%s-%s@kubernetescrd" .ctx.Release.Namespace (include "phase-server.fullname" .ctx) .suffix -}}
{{- end -}}

{{/* Middlewares applied to every public route */}}
{{- define "phase-server.commonMiddlewares" -}}
{{- $list := list -}}
{{- if .Values.traefik.middlewares.enabled -}}
{{- $list = append $list (include "phase-server.crdRef" (dict "ctx" . "suffix" "ratelimit")) -}}
{{- $list = append $list (include "phase-server.crdRef" (dict "ctx" . "suffix" "inflight")) -}}
{{- $list = append $list (include "phase-server.crdRef" (dict "ctx" . "suffix" "headers")) -}}
{{- end -}}
{{- $list = concat $list (default (list) .Values.traefik.middlewares.extra) -}}
{{- join "," $list -}}
{{- end -}}

{{/* Traefik source criterion for the per-source middlewares */}}
{{- define "phase-server.sourceCriterion" -}}
{{- if .Values.cloudflare.enabled -}}
{{- if not (or .Values.cloudflare.authenticatedOriginPulls.enabled .Values.cloudflare.trustHeaderWithoutOriginPulls) -}}
{{- fail "cloudflare.enabled keys rate limits on CF-Connecting-IP, which anyone reaching the origin directly can forge (each forged value gets its own bucket). Enable cloudflare.authenticatedOriginPulls, or set cloudflare.trustHeaderWithoutOriginPulls=true if a firewall/Tunnel already restricts the origin to Cloudflare." -}}
{{- end -}}
requestHeaderName: CF-Connecting-IP
{{- else -}}
{{- toYaml .Values.traefik.middlewares.sourceCriterion -}}
{{- end -}}
{{- end -}}

{{- define "phase-server.ingressAnnotations" -}}
{{- with .Values.ingress.annotations }}
{{- toYaml . }}
{{ end -}}
{{- if .Values.cloudflare.authenticatedOriginPulls.enabled }}
traefik.ingress.kubernetes.io/router.tls.options: {{ include "phase-server.crdRef" (dict "ctx" . "suffix" "cf-origin-pull") | quote }}
{{- end }}
{{- end -}}
