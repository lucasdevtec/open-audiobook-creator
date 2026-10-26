#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MODELS_DIR="${ROOT_DIR}/models"
BIN_DIR="${ROOT_DIR}/bin"

mkdir -p "${MODELS_DIR}" "${BIN_DIR}"

echo "================================================================"
echo "🎙️  Instalador de Motores TTS Locais — Open Audiobook Creator"
echo "================================================================"
echo "Diretório de modelos: ${MODELS_DIR}"
echo "Diretório de binários: ${BIN_DIR}"
echo ""

# ------------------------------------------------------------------------------
# 1. PIPER TTS (Binário + Modelos ONNX em Português e Inglês)
# ------------------------------------------------------------------------------
install_piper() {
  echo "➡️  [1/3] Configurando Piper TTS..."
  OS="$(uname -s)"
  ARCH="$(uname -m)"

  if [[ "${OS}" == "Linux" ]]; then
    if [[ "${ARCH}" == "x86_64" ]]; then
      PIPER_ARCHIVE="piper_linux_x86_64.tar.gz"
    elif [[ "${ARCH}" == "aarch64" || "${ARCH}" == "arm64" ]]; then
      PIPER_ARCHIVE="piper_linux_aarch64.tar.gz"
    else
      echo "⚠️  Arquitetura ${ARCH} não possui binário pré-compilado padrão do Piper."
      PIPER_ARCHIVE=""
    fi

    if [[ -n "${PIPER_ARCHIVE}" && ! -f "${BIN_DIR}/piper/piper" ]]; then
      PIPER_URL="https://github.com/rhasspy/piper/releases/download/2023.11.14-2/${PIPER_ARCHIVE}"
      echo "   Baixando binário Piper (${PIPER_ARCHIVE})..."
      curl -L --fail --progress-bar "${PIPER_URL}" -o "/tmp/${PIPER_ARCHIVE}"
      tar -xzf "/tmp/${PIPER_ARCHIVE}" -C "${BIN_DIR}"
      rm -f "/tmp/${PIPER_ARCHIVE}"
      ln -sf "${BIN_DIR}/piper/piper" "${BIN_DIR}/piper-tts"
      echo "   ✅ Binário Piper instalado em ${BIN_DIR}/piper/piper"
    else
      echo "   ✅ Binário Piper já presente."
    fi
  elif [[ "${OS}" == "Darwin" ]]; then
    echo "   ℹ️  No macOS, instale o Piper via Python (.venv) ou Homebrew."
  fi

  # Download das vozes ONNX oficiais (Hugging Face rhasspy/piper-voices)
  HF_PIPER_BASE="https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0"

  download_piper_voice() {
    local rel_path="$1"
    local filename="$2"
    if [[ ! -f "${MODELS_DIR}/${filename}.onnx" ]]; then
      echo "   Baixando voz Piper: ${filename}..."
      curl -L --fail --progress-bar "${HF_PIPER_BASE}/${rel_path}/${filename}.onnx" \
        -o "${MODELS_DIR}/${filename}.onnx"
      curl -L --fail -s "${HF_PIPER_BASE}/${rel_path}/${filename}.onnx.json" \
        -o "${MODELS_DIR}/${filename}.onnx.json"
    else
      echo "   ✅ Voz ${filename} já existe em models/."
    fi
  }

  download_piper_voice "pt/pt_BR/faber/medium" "pt_BR-faber-medium"
  download_piper_voice "pt/pt_BR/edresson/low" "pt_BR-edresson-low"
  download_piper_voice "en/en_US/lessac/high" "en_US-lessac-high"
  echo ""
}

# ------------------------------------------------------------------------------
# 2. KOKORO-82M ONNX (kokoro-onnx + aceleração GPU ROCm/CUDA + wrapper CLI)
# ------------------------------------------------------------------------------
install_kokoro() {
  echo "➡️  [2/3] Configurando Kokoro-82M ONNX..."
  KOKORO_RELEASE_BASE="https://github.com/thewh1teagle/kokoro-onnx/releases/download/model-files-v1.0"

  if [[ ! -f "${MODELS_DIR}/kokoro-v1_0.onnx" ]]; then
    echo "   Baixando modelo neural kokoro-v1_0.onnx (~310 MB)..."
    curl -L --fail --progress-bar "${KOKORO_RELEASE_BASE}/kokoro-v1_0.onnx" \
      -o "${MODELS_DIR}/kokoro-v1_0.onnx"
  else
    echo "   ✅ kokoro-v1_0.onnx já presente."
  fi

  if [[ ! -f "${MODELS_DIR}/voices-v1.0.bin" ]]; then
    echo "   Baixando pacote de vozes voices-v1.0.bin..."
    curl -L --fail --progress-bar "${KOKORO_RELEASE_BASE}/voices-v1.0.bin" \
      -o "${MODELS_DIR}/voices-v1.0.bin"
  else
    echo "   ✅ voices-v1.0.bin já presente."
  fi

  if command -v python3 >/dev/null 2>&1; then
    if [[ ! -d "${ROOT_DIR}/.venv" ]]; then
      echo "   Criando ambiente virtual Python em ${ROOT_DIR}/.venv..."
      python3 -m venv "${ROOT_DIR}/.venv"
    fi
    echo "   Instalando runtime kokoro-onnx e soundfile no .venv..."
    "${ROOT_DIR}/.venv/bin/pip" install --upgrade pip -q
    "${ROOT_DIR}/.venv/bin/pip" install kokoro-onnx soundfile -q

    # Detectar GPU AMD (ROCm) ou NVIDIA (CUDA) para habilitar aceleração por GPU
    if [[ -e "/dev/kfd" ]]; then
      echo "   🚀 GPU AMD ROCm (/dev/kfd) detectada! Instalando onnxruntime-rocm..."
      "${ROOT_DIR}/.venv/bin/pip" uninstall -y onnxruntime -q >/dev/null 2>&1 || true
      "${ROOT_DIR}/.venv/bin/pip" install onnxruntime-rocm -q

      if [[ ! -f "${BIN_DIR}/rocm-libs/libMIOpen.so.1" && ! -f "/opt/rocm/lib/libMIOpen.so.1" ]]; then
        echo "   Baixando bibliotecas auxiliares do ROCm 7.2 (MIOpen, rocFFT, hipFFT, hiprtc)..."
        mkdir -p "${BIN_DIR}/rocm-libs" /tmp/rocm-debs
        ROCM_APT="https://repo.radeon.com/rocm/apt/7.2/pool/main"
        curl -sL "${ROCM_APT}/h/hipfft/hipfft_1.0.22.70200-43~24.04_amd64.deb" -o /tmp/rocm-debs/hipfft.deb
        curl -sL "${ROCM_APT}/r/rocm-smi-lib/rocm-smi-lib_7.8.0.70200-43~24.04_amd64.deb" -o /tmp/rocm-debs/rocm-smi.deb
        curl -sL "${ROCM_APT}/r/roctracer/roctracer_4.1.70200.70200-43~24.04_amd64.deb" -o /tmp/rocm-debs/roctracer.deb
        curl -sL "${ROCM_APT}/r/rocm-core/rocm-core_7.2.0.70200-43~24.04_amd64.deb" -o /tmp/rocm-debs/rocm-core.deb
        curl -sL "${ROCM_APT}/h/hip-runtime-amd/hip-runtime-amd_7.2.26015.70200-43~24.04_amd64.deb" -o /tmp/rocm-debs/hip-runtime.deb
        curl -L --progress-bar "${ROCM_APT}/r/rocfft/rocfft_1.0.36.70200-43~24.04_amd64.deb" -o /tmp/rocm-debs/rocfft.deb
        curl -L --progress-bar "${ROCM_APT}/m/miopen-hip/miopen-hip_3.5.1.70200-43~24.04_amd64.deb" -o /tmp/rocm-debs/miopen.deb
        for d in /tmp/rocm-debs/*.deb; do dpkg-deb -x "$d" /tmp/rocm-debs/extracted; done
        cp -a /tmp/rocm-debs/extracted/opt/rocm*/lib/*.so* "${BIN_DIR}/rocm-libs/"
        rm -rf /tmp/rocm-debs
      fi
    elif command -v nvidia-smi >/dev/null 2>&1; then
      echo "   🚀 GPU NVIDIA detectada! Instalando onnxruntime-gpu..."
      "${ROOT_DIR}/.venv/bin/pip" uninstall -y onnxruntime -q >/dev/null 2>&1 || true
      "${ROOT_DIR}/.venv/bin/pip" install onnxruntime-gpu -q
    fi

    # Criar wrapper executável ./bin/kokoro-tts com suporte ROCm/CUDA e worker persistente
    cat << 'EOF' > "${BIN_DIR}/kokoro-tts"
#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

ROCM_PATHS="${ROOT_DIR}/bin/rocm-libs"
if [[ -d "/usr/local/lib/ollama/rocm_v7_2" ]]; then
  ROCM_PATHS="${ROCM_PATHS}:/usr/local/lib/ollama/rocm_v7_2"
fi
if [[ -d "/opt/rocm/lib" ]]; then
  ROCM_PATHS="${ROCM_PATHS}:/opt/rocm/lib"
fi

export LD_LIBRARY_PATH="${ROCM_PATHS}:${LD_LIBRARY_PATH:-}"
export HSA_OVERRIDE_GFX_VERSION="${HSA_OVERRIDE_GFX_VERSION:-11.0.0}"
export MIOPEN_FIND_MODE="${MIOPEN_FIND_MODE:-FAST}"
export MIOPEN_LOG_LEVEL="${MIOPEN_LOG_LEVEL:-3}"
export ORT_LOG_SEVERITY_LEVEL="${ORT_LOG_SEVERITY_LEVEL:-3}"

exec "${ROOT_DIR}/.venv/bin/python" "${ROOT_DIR}/scripts/kokoro_bridge.py" "$@"
EOF
    chmod +x "${BIN_DIR}/kokoro-tts"
    echo "   ✅ Wrapper CLI Kokoro configurado em ${BIN_DIR}/kokoro-tts"
  else
    echo "   ⚠️  python3 não encontrado. Instale o Python 3.10+ para habilitar o wrapper kokoro-onnx."
  fi
  echo ""
}

# ------------------------------------------------------------------------------
# 3. COQUI XTTS-v2 (Opcional — pesado, recomendado apenas se usar clonagem de voz)
# ------------------------------------------------------------------------------
install_xtts() {
  if [[ "${1:-}" == "--with-xtts" ]]; then
    echo "➡️  [3/3] Instalando Coqui XTTS-v2 (coqui-tts)..."
    "${ROOT_DIR}/.venv/bin/pip" install coqui-tts -q
    echo "   ✅ Coqui XTTS instalado em .venv/bin/tts"
  else
    echo "ℹ️  [3/3] Coqui XTTS-v2 pulado por padrão (use './scripts/install-tts-models.sh --with-xtts' se desejar clonagem de voz)."
  fi
}

install_piper
install_kokoro
install_xtts "${1:-}"

echo "================================================================"
echo "🎉 Instalação concluída! Verifique o diagnóstico rodando:"
echo "   cargo run -p oac-cli -- voices"
echo "================================================================"
