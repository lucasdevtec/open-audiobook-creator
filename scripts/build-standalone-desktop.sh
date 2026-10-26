#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT_DIR}"
mkdir -p "${ROOT_DIR}/dist"

echo "================================================================"
echo "📦 Empacotador Desktop Nativo (.AppImage + .exe) — Open Audiobook Creator"
echo "================================================================"

# 1. Compilar Frontend React (ui/dist) para embutir no executável Rust
echo "➡️  [1/4] Compilando interface gráfica (React + Vite)..."
npm --prefix "${ROOT_DIR}/ui" run build

# 2. Compilar binários Linux Release (oac-desktop nativo + oac-cli)
echo "➡️  [2/4] Compilando binários Linux x86_64 Release..."
cargo build --release --workspace

# 3. Empacotar .AppImage (Linux — Janela Desktop Nativa GTK/WebKit sem servidor HTTP)
echo "➡️  [3/4] Gerando pacote dist/Open-Audiobook-Creator-0.1.0-x86_64.AppImage..."
rm -rf /tmp/Oac.AppDir
mkdir -p /tmp/Oac.AppDir/usr/bin
cp "${ROOT_DIR}/target/release/open-audiobook-creator" /tmp/Oac.AppDir/usr/bin/open-audiobook-creator
cp "${ROOT_DIR}/target/release/oac" /tmp/Oac.AppDir/usr/bin/oac

cat << 'EOF' > /tmp/Oac.AppDir/AppRun
#!/usr/bin/env bash
HERE="$(dirname "$(readlink -f "${0}")")"
export PATH="${HERE}/usr/bin:${PATH}"
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

# 4. Cross-compile Windows 64-bit (.exe Nativo GUI sem console e sem servidor HTTP) via cargo-zigbuild
echo "➡️  [4/4] Compilando aplicativo Desktop Windows 64-bit (dist/Open-Audiobook-Creator-0.1.0-x64.exe)..."
rustup target add x86_64-pc-windows-gnu
if [[ -d "${ROOT_DIR}/.venv" ]]; then
  "${ROOT_DIR}/.venv/bin/pip" install ziglang cargo-zigbuild -q
  PATH="${ROOT_DIR}/.venv/bin:${PATH}" cargo zigbuild --release --target x86_64-pc-windows-gnu -p oac-desktop
else
  cargo zigbuild --release --target x86_64-pc-windows-gnu -p oac-desktop
fi
cp "${ROOT_DIR}/target/x86_64-pc-windows-gnu/release/open-audiobook-creator.exe" "${ROOT_DIR}/dist/Open-Audiobook-Creator-0.1.0-x64.exe"

echo "================================================================"
echo "🎉 Pacotes Desktop Nativos gerados com sucesso em dist/:"
ls -lh "${ROOT_DIR}/dist/"
echo "================================================================"
