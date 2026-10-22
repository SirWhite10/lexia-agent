$ErrorActionPreference = 'Stop'

$modelName = 'sherpa-onnx-kws-zipformer-gigaspeech-3.3M-2024-01-01'
$modelDir = Join-Path 'models' $modelName
$archive = Join-Path 'models' "$modelName.tar.bz2"

New-Item -ItemType Directory -Force -Path 'models', 'assets' | Out-Null
if (-not (Test-Path -LiteralPath $modelDir)) {
	$url = "https://github.com/k2-fsa/sherpa-onnx/releases/download/kws-models/$modelName.tar.bz2"
	Invoke-WebRequest -Uri $url -OutFile $archive
	tar -xjf $archive -C models
	Remove-Item -LiteralPath $archive
}

Write-Host "Prepared model at $modelDir; the bundled token list listens for LEXIA."
