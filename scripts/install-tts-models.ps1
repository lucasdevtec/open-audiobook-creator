# Instalador PowerShell para Windows — Open Audiobook Creator (Piper TTS + Kokoro-82M ONNX)
$ErrorActionPreference = "Stop"

$RootDir = Split-Path -Parent $PSScriptRoot
$ModelsDir = Join-Path $RootDir "models"
$BinDir = Join-Path $RootDir "bin"

New-Item -ItemType Directory -Force -Path $ModelsDir | Out-Null
New-Item -ItemType Directory -Force -Path $BinDir | Out-Null

Write-Host "================================================================" -ForegroundColor Cyan
Write-Host "🎙️  Instalador de Motores TTS Locais (Windows)" -ForegroundColor Cyan
Write-Host "================================================================" -ForegroundColor Cyan

# 1. Piper TTS (Windows x64)
$PiperExe = Join-Path $BinDir "piper\piper.exe"
if (-Not (Test-Path $PiperExe)) {
    $PiperZip = Join-Path $env:TEMP "piper_windows_amd64.zip"
    $PiperUrl = "https://github.com/rhasspy/piper/releases/download/2023.11.14-2/piper_windows_amd64.zip"
    Write-Host "Baixando Piper TTS para Windows..."
    Invoke-WebRequest -Uri $PiperUrl -OutFile $PiperZip
    Expand-Archive -Path $PiperZip -DestinationPath $BinDir -Force
    Remove-Item $PiperZip -Force
    Write-Host "✅ Piper instalado em $PiperExe" -ForegroundColor Green
}

# Vozes Piper ONNX
$HfBase = "https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0"
$Voices = @(
    @{ Path = "pt/pt_BR/faber/medium"; Name = "pt_BR-faber-medium" },
    @{ Path = "pt/pt_BR/edresson/low"; Name = "pt_BR-edresson-low" },
    @{ Path = "en/en_US/lessac/high"; Name = "en_US-lessac-high" }
)

foreach ($v in $Voices) {
    $OnnxFile = Join-Path $ModelsDir "$($v.Name).onnx"
    $JsonFile = Join-Path $ModelsDir "$($v.Name).onnx.json"
    if (-Not (Test-Path $OnnxFile)) {
        Write-Host "Baixando voz Piper: $($v.Name)..."
        Invoke-WebRequest -Uri "$HfBase/$($v.Path)/$($v.Name).onnx" -OutFile $OnnxFile
        Invoke-WebRequest -Uri "$HfBase/$($v.Path)/$($v.Name).onnx.json" -OutFile $JsonFile
    }
}

# 2. Kokoro-82M ONNX
$KokoroBase = "https://github.com/thewh1teagle/kokoro-onnx/releases/download/model-files-v1.0"
$KokoroModel = Join-Path $ModelsDir "kokoro-v1_0.onnx"
$KokoroVoices = Join-Path $ModelsDir "voices-v1.0.bin"

if (-Not (Test-Path $KokoroModel)) {
    Write-Host "Baixando modelo Kokoro-82M (kokoro-v1_0.onnx)..."
    Invoke-WebRequest -Uri "$KokoroBase/kokoro-v1.0.onnx" -OutFile $KokoroModel
}
if (-Not (Test-Path $KokoroVoices)) {
    Write-Host "Baixando vozes Kokoro (voices-v1.0.bin)..."
    Invoke-WebRequest -Uri "$KokoroBase/voices-v1.0.bin" -OutFile $KokoroVoices
}

if (Get-Command python -ErrorAction SilentlyContinue) {
    $VenvDir = Join-Path $RootDir ".venv"
    if (-Not (Test-Path $VenvDir)) {
        python -m venv $VenvDir
    }
    & "$VenvDir\Scripts\pip.exe" install --upgrade pip kokoro-onnx soundfile

    $CmdWrapper = Join-Path $BinDir "kokoro-tts.cmd"
    "@echo off`r`n`"%~dp0..\.venv\Scripts\python.exe`" `"%~dp0..\scripts\kokoro_bridge.py`" %*" | Set-Content -Path $CmdWrapper -Encoding ASCII
    Write-Host "✅ Kokoro-82M configurado em $CmdWrapper" -ForegroundColor Green
}

Write-Host "🎉 Concluído! Execute: cargo run -p oac-cli -- voices" -ForegroundColor Green

