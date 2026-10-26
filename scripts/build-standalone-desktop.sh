#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT_DIR}"
mkdir -p "${ROOT_DIR}/dist"

echo "================================================================"
echo "📦 Empacotador Standalone All-in-One (.AppImage + .exe)"
echo "   Tanto o .AppImage quanto o .exe carregam Kokoro-82M + Piper + Modelos"
echo "================================================================"

# 0. Garantir que modelos e binários locais existam antes de empacotar
if [[ ! -f "${ROOT_DIR}/models/kokoro-v1_0.onnx" || ! -d "${ROOT_DIR}/bin/piper" ]]; then
  echo "➡️  [0/4] Baixando modelos e motores TTS locais para embutir..."
  bash "${ROOT_DIR}/scripts/install-tts-models.sh"
fi

# 1. Compilar Frontend React (ui/dist) para embutir no executável Rust
echo "➡️  [1/4] Compilando interface gráfica (React + Vite)..."
npm --prefix "${ROOT_DIR}/ui" run build

# 2. Compilar binários Linux Release (oac-desktop nativo + oac-cli)
echo "➡️  [2/4] Compilando binários Linux x86_64 Release..."
cargo build --release --workspace

# 3. Empacotar .AppImage All-in-One (Binários + Runtime Kokoro/Piper + Modelos ONNX embutidos)
echo "➡️  [3/4] Montando AppImage Linux All-in-One..."
rm -rf /tmp/Oac.AppDir
RES_DIR="/tmp/Oac.AppDir/usr/lib/open-audiobook-creator"
mkdir -p /tmp/Oac.AppDir/usr/bin "${RES_DIR}/bin" "${RES_DIR}/models" "${RES_DIR}/scripts"

cp "${ROOT_DIR}/target/release/open-audiobook-creator" /tmp/Oac.AppDir/usr/bin/open-audiobook-creator
cp "${ROOT_DIR}/target/release/oac" /tmp/Oac.AppDir/usr/bin/oac

# Copiar binários Piper TTS e wrappers CLI
cp -a "${ROOT_DIR}/bin/piper" "${RES_DIR}/bin/"
cp -a "${ROOT_DIR}/bin/piper-tts" "${RES_DIR}/bin/"
cp -a "${ROOT_DIR}/bin/kokoro-tts" "${RES_DIR}/bin/"
chmod +x "${RES_DIR}/bin/kokoro-tts" "${RES_DIR}/bin/piper-tts" "${RES_DIR}/bin/piper/piper"
cp "${ROOT_DIR}/scripts/kokoro_bridge.py" "${RES_DIR}/scripts/kokoro_bridge.py"
chmod +x "${RES_DIR}/scripts/kokoro_bridge.py"

# Embutir Python 3.12 Standalone e isolado do sistema hospedeiro
echo "   Embutindo runtime Python 3.12 Standalone para Linux x86_64..."
if [[ ! -f /tmp/cpython_linux_x64.tar.gz ]]; then
  curl -sL "https://github.com/astral-sh/python-build-standalone/releases/download/20250212/cpython-3.12.9+20250212-x86_64-unknown-linux-gnu-install_only_stripped.tar.gz" -o /tmp/cpython_linux_x64.tar.gz
fi
tar -xzf /tmp/cpython_linux_x64.tar.gz -C "${RES_DIR}"

# Copiar bibliotecas Python do Kokoro (kokoro-onnx, onnxruntime CPU, espeakng) para o Python embutido
SITE_PKGS="$(find "${ROOT_DIR}/.venv/lib" -maxdepth 3 -name "site-packages" | head -n 1)"
if [[ -n "${SITE_PKGS}" && -d "${SITE_PKGS}" ]]; then
  echo "   Embutindo módulos Kokoro-82M no Python portátil..."
  mkdir -p "${RES_DIR}/python/lib/python3.12/site-packages"
  rsync -a \
    --exclude='ziglang*' \
    --exclude='cargo_zigbuild*' \
    --exclude='pip*' \
    --exclude='*rocm*.so*' \
    --exclude='__pycache__' \
    "${SITE_PKGS}/" "${RES_DIR}/python/lib/python3.12/site-packages/"
fi

# Copiar todos os modelos ONNX (Kokoro-82M + Vozes Piper PT-BR/EN)
echo "   Embutindo modelos de voz ONNX (Kokoro-82M + Piper PT-BR/EN)..."
cp -f "${ROOT_DIR}"/models/*.onnx* "${RES_DIR}/models/"
cp -f "${ROOT_DIR}/models/voices-v1.0.bin" "${RES_DIR}/models/"

cat << 'EOF' > /tmp/Oac.AppDir/AppRun
#!/usr/bin/env bash
HERE="$(dirname "$(readlink -f "${0}")")"
export OAC_RESOURCE_DIR="${HERE}/usr/lib/open-audiobook-creator"
export PATH="${OAC_RESOURCE_DIR}/bin:${OAC_RESOURCE_DIR}/bin/piper:${OAC_RESOURCE_DIR}/python/bin:${HERE}/usr/bin:${PATH}"
export PYTHONIOENCODING="utf-8"
export PYTHONUTF8="1"
export LC_ALL="${LC_ALL:-C.UTF-8}"
export LANG="${LANG:-C.UTF-8}"
export WEBKIT_DISABLE_DMABUF_RENDERER="${WEBKIT_DISABLE_DMABUF_RENDERER:-1}"
if [[ $# -gt 0 ]]; then
  exec "${HERE}/usr/bin/oac" "$@"
else
  exec "${HERE}/usr/bin/open-audiobook-creator"
fi
EOF
chmod +x /tmp/Oac.AppDir/AppRun

cat << 'EOF' > /tmp/Oac.AppDir/open-audiobook-creator.desktop
[Desktop Entry]
Name=Open Audiobook Creator
Exec=open-audiobook-creator
Icon=open-audiobook-creator
Type=Application
Categories=AudioVideo;
Terminal=false
EOF

cat << 'EOF' > /tmp/Oac.AppDir/open-audiobook-creator.svg
<svg xmlns="http://www.w3.org/2000/svg" width="256" height="256" viewBox="0 0 256 256">
  <rect width="256" height="256" rx="56" fill="#4f46e5"/>
  <path d="M64 144v-16a64 64 0 0 1 128 0v16" fill="none" stroke="#ffffff" stroke-width="18" stroke-linecap="round"/>
  <rect x="48" y="136" width="36" height="56" rx="14" fill="#ffffff"/>
  <rect x="172" y="136" width="36" height="56" rx="14" fill="#ffffff"/>
</svg>
EOF

if [[ ! -f /tmp/appimagetool ]]; then
  curl -sL "https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-x86_64.AppImage" -o /tmp/appimagetool
  chmod +x /tmp/appimagetool
fi
ARCH=x86_64 /tmp/appimagetool --appimage-extract-and-run /tmp/Oac.AppDir "${ROOT_DIR}/dist/Open-Audiobook-Creator-0.1.0-x86_64.AppImage"
chmod +x "${ROOT_DIR}/dist/Open-Audiobook-Creator-0.1.0-x86_64.AppImage"

# 4. Cross-compile Windows 64-bit (.exe All-in-One com Piper Windows + Python Portátil + Kokoro + Modelos ONNX embutidos)
echo "➡️  [4/4] Montando aplicativo Desktop Windows All-in-One (dist/Open-Audiobook-Creator-0.1.0-x64.exe)..."
rustup target add x86_64-pc-windows-gnu
if [[ -d "${ROOT_DIR}/.venv" ]]; then
  "${ROOT_DIR}/.venv/bin/pip" install ziglang cargo-zigbuild -q
  PATH="${ROOT_DIR}/.venv/bin:${PATH}" cargo zigbuild --release --target x86_64-pc-windows-gnu -p oac-desktop
else
  cargo zigbuild --release --target x86_64-pc-windows-gnu -p oac-desktop
fi

WIN_BUNDLE="/tmp/Oac.WinBundle"
rm -rf "${WIN_BUNDLE}" /tmp/oac_win_payload.zip
mkdir -p "${WIN_BUNDLE}/bin" "${WIN_BUNDLE}/models" "${WIN_BUNDLE}/scripts"

# 4.1 Baixar Piper TTS Windows x64 se não estiver em cache
if [[ ! -f /tmp/piper_windows_amd64.zip ]]; then
  echo "   Baixando binário Piper TTS para Windows x64..."
  curl -sL "https://github.com/rhasspy/piper/releases/download/2023.11.14-2/piper_windows_amd64.zip" -o /tmp/piper_windows_amd64.zip
fi
unzip -q -o /tmp/piper_windows_amd64.zip -d "${WIN_BUNDLE}/bin"

# 4.2 Baixar Python 3.12 Portátil para Windows x64 + wheels nativas win_amd64 do Kokoro-82M
if [[ ! -f /tmp/cpython_win_x64.tar.gz ]]; then
  echo "   Baixando Python 3.12 Portátil para Windows x64..."
  curl -sL "https://github.com/astral-sh/python-build-standalone/releases/download/20250212/cpython-3.12.9+20250212-x86_64-pc-windows-msvc-install_only_stripped.tar.gz" -o /tmp/cpython_win_x64.tar.gz
fi
tar -xzf /tmp/cpython_win_x64.tar.gz -C "${WIN_BUNDLE}"

mkdir -p /tmp/win-wheels
if [[ -z "$(ls -A /tmp/win-wheels 2>/dev/null)" ]]; then
  echo "   Baixando pacotes Windows (win_amd64) do Kokoro-82M (kokoro-onnx, onnxruntime, soundfile, espeakng)..."
  "${ROOT_DIR}/.venv/bin/pip" download --dest /tmp/win-wheels --platform win_amd64 --python-version 3.12 --only-binary=:all: kokoro-onnx onnxruntime soundfile -q
fi
mkdir -p "${WIN_BUNDLE}/python/Lib/site-packages"
for whl in /tmp/win-wheels/*.whl; do
  unzip -q -o "${whl}" -d "${WIN_BUNDLE}/python/Lib/site-packages"
done

# 4.3 Criar wrapper kokoro-tts.cmd e copiar kokoro_bridge.py + modelos ONNX
cp "${ROOT_DIR}/scripts/kokoro_bridge.py" "${WIN_BUNDLE}/scripts/kokoro_bridge.py"
cat << 'EOF' > "${WIN_BUNDLE}/bin/kokoro-tts.cmd"
@echo off
set "PYTHONIOENCODING=utf-8"
set "PYTHONUTF8=1"
"%~dp0..\python\python.exe" "%~dp0..\scripts\kokoro_bridge.py" %*
EOF

cp -f "${ROOT_DIR}"/models/*.onnx* "${WIN_BUNDLE}/models/"
cp -f "${ROOT_DIR}/models/voices-v1.0.bin" "${WIN_BUNDLE}/models/"

echo "   Compactando payload Windows All-in-One e acoplando ao executável .exe..."
(cd "${WIN_BUNDLE}" && zip -q -1 -r /tmp/oac_win_payload.zip bin python scripts models)
cat "${ROOT_DIR}/target/x86_64-pc-windows-gnu/release/open-audiobook-creator.exe" /tmp/oac_win_payload.zip > "${ROOT_DIR}/dist/Open-Audiobook-Creator-0.1.0-x64.exe"
chmod +x "${ROOT_DIR}/dist/Open-Audiobook-Creator-0.1.0-x64.exe"
rm -rf "${WIN_BUNDLE}" /tmp/oac_win_payload.zip

echo "================================================================"
echo "🎉 Pacotes Desktop All-in-One (.AppImage + .exe) gerados em dist/:"
ls -lh "${ROOT_DIR}/dist/"
echo "================================================================"
