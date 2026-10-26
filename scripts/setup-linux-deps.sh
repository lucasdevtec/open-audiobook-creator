#!/usr/bin/env bash
set -euo pipefail

echo "📦 Instalando dependências de sistema para Tauri v2, FFmpeg, Kokoro e Piper TTS no Linux (Debian/Ubuntu)..."
sudo apt-get update
sudo apt-get install -y \
  libwebkit2gtk-4.1-dev \
  build-essential \
  curl \
  wget \
  file \
  libxdo-dev \
  libssl-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev \
  ffmpeg \
  espeak-ng \
  libsndfile1 \
  python3-venv \
  python3-pip

echo "✅ Dependências de sistema instaladas com sucesso!"
echo "👉 Próximo passo: execute './scripts/install-tts-models.sh' para baixar o Kokoro-82M e o Piper TTS."
