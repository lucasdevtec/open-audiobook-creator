# 🛠️ Guia Passo a Passo de Instalação das Dependências e Motores TTS (Kokoro, Piper, XTTS e FFmpeg)

Este guia detalha como instalar todas as dependências de sistema e os motores de síntese de voz (TTS) locais e gratuitos (**Kokoro-82M ONNX**, **Piper TTS** e **Coqui XTTS-v2**) no **Linux**, **Windows** e **macOS**.

---

## ⚡ Opção Rápida (Instalador Automático de Vozes e Motores)

O repositório inclui scripts que baixam e configuram automaticamente o **Piper TTS**, o **Kokoro-82M ONNX** (`kokoro-v1_0.onnx` + `voices-v1.0.bin` + ponte CLI) e as vozes em Português (`pt-BR`) e Inglês (`en-US`) dentro das pastas `./bin` e `./models` do projeto:

- **No Linux ou macOS**:
  ```bash
  chmod +x scripts/install-tts-models.sh
  ./scripts/install-tts-models.sh
  ```
- **No Windows (PowerShell)**:
  ```powershell
  powershell -ExecutionPolicy Bypass -File .\scripts\install-tts-models.ps1
  ```

Após executar o script, rode:
```bash
cargo run -p oac-cli -- voices
```
O sistema detectará automaticamente os binários em `./bin/` e os modelos em `./models/`.

---

## 1. Pré-requisitos Gerais do Sistema (Rust, Node.js e FFmpeg)

### 🐧 Linux (Ubuntu / Debian / Pop!_OS / Mint)
Para instalar o `ffmpeg` (codificação AAC/M4B e LAME/MP3), `espeak-ng` (fonemizador auxiliar do Kokoro) e as bibliotecas do WebKitGTK (para abrir a janela nativa Tauri):

```bash
sudo apt-get update
sudo apt-get install -y \
  ffmpeg \
  espeak-ng \
  libsndfile1 \
  python3-venv \
  python3-pip \
  libwebkit2gtk-4.1-dev \
  build-essential \
  curl \
  wget \
  file \
  libxdo-dev \
  libssl-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev
```
*(Ou execute simplesmente `./scripts/setup-linux-deps.sh`).*

### 🪟 Windows (10 / 11)
1. **Rust**: Instale via [rustup.rs](https://rustup.rs/) (requer *Visual Studio C++ Build Tools*).
2. **Node.js (v20+)**: Instale via `winget install OpenJS.NodeJS.LTS`.
3. **FFmpeg & Python 3.11+**:
   ```powershell
   winget install Gyan.FFmpeg
   winget install Python.Python.3.12
   ```

### 🍎 macOS
Usando o [Homebrew](https://brew.sh/):
```bash
brew install rust node ffmpeg espeak-ng libsndfile python@3.12
```

---

## 2. Passo a Passo Manual: Kokoro-82M ONNX (Vozes Ultra-Naturais em Português e Inglês)

O **Kokoro-82M** é um dos melhores modelos abertos de TTS da atualidade: possui apenas 82 milhões de parâmetros, roda localmente em CPU com rapidez via **ONNX Runtime** e conta com vozes nativas em **Português do Brasil** (`pf_dora` feminina e `pm_alex` masculino) e **Inglês** (`af_heart`, `am_adam`).

### Passo 2.1 — Baixar os arquivos do modelo ONNX para a pasta `models/`
Na raiz do projeto `open-audiobook-creator`, crie a pasta `models` e baixe o modelo `kokoro-v1_0.onnx` (~310 MB) e o pacote de vozes `voices-v1.0.bin`:

```bash
mkdir -p models bin

# 1. Baixar o modelo neural ONNX (kokoro-v1_0.onnx)
curl -L https://github.com/thewh1teagle/kokoro-onnx/releases/download/model-files-v1.0/kokoro-v1_0.onnx \
  -o models/kokoro-v1_0.onnx

# 2. Baixar o pacote de vetores de vozes (voices-v1.0.bin)
curl -L https://github.com/thewh1teagle/kokoro-onnx/releases/download/model-files-v1.0/voices-v1.0.bin \
  -o models/voices-v1.0.bin
```

### Passo 2.2 — Criar o ambiente e instalar o `kokoro-onnx`
O pacote `kokoro-onnx` executa o modelo ONNX diretamente em CPU (ou GPU) sem precisar do PyTorch completo:

```bash
# Criar ambiente virtual na raiz do projeto
python3 -m venv .venv

# Instalar kokoro-onnx e soundfile
./.venv/bin/pip install --upgrade pip
./.venv/bin/pip install kokoro-onnx soundfile
```

> **Aceleração por GPU no Kokoro-82M (AMD ROCm & NVIDIA CUDA)**:
> - **AMD Radeon (ROCm no Linux)**: O script `./scripts/install-tts-models.sh` detecta automaticamente `/dev/kfd`, instala o `onnxruntime-rocm`, aproveita as bibliotecas ROCm 7.2 do Ollama (`/usr/local/lib/ollama/rocm_v7_2`) + bibliotecas `MIOpen`/`rocFFT` em `./bin/rocm-libs`, define `HSA_OVERRIDE_GFX_VERSION=11.0.0` e `MIOPEN_FIND_MODE=FAST`, e mantém um worker persistente na VRAM (gerando 10s de áudio em ~1.6s na Radeon RX 7600!).
> - **NVIDIA (CUDA)**: Se `nvidia-smi` for detectado, instala automaticamente o `onnxruntime-gpu`.
> - **Windows (AMD / Intel / NVIDIA)**: Substitua `onnxruntime` por `onnxruntime-directml` (`./.venv/Scripts/pip install onnxruntime-directml`).

### Passo 2.3 — Habilitar o executável `bin/kokoro-tts`
O Open Audiobook Creator já inclui o script de ponte [`scripts/kokoro_bridge.py`](../scripts/kokoro_bridge.py). Basta criar o lançador em `bin/kokoro-tts`:

```bash
cat << 'EOF' > bin/kokoro-tts
#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
exec "${ROOT_DIR}/.venv/bin/python" "${ROOT_DIR}/scripts/kokoro_bridge.py" "$@"
EOF

chmod +x bin/kokoro-tts
```

### Passo 2.4 — Testar o Kokoro isoladamente e no Open Audiobook Creator
Você pode testar a síntese do Kokoro em Português diretamente no terminal:
```bash
echo "Olá! Este é um teste da voz neural Kokoro em português do Brasil." | \
  ./bin/kokoro-tts --voice pf_dora --speed 1.0 --output teste_kokoro.wav
```
E para gerar um audiobook completo usando o **Kokoro**:
```bash
cargo run -p oac-cli -- render \
  --input ./output/livro_exemplo.pdf \
  --engine kokoro \
  --voice kokoro-pf_dora \
  --format m4b
```

#### Tabela de Vozes Populares do Kokoro-82M:
| Código da Voz | Idioma | Gênero | Estilo |
| :--- | :--- | :--- | :--- |
| `pf_dora` | Português (`pt-BR`) | Feminino | Clara, expressiva, excelente para livros |
| `pm_alex` | Português (`pt-BR`) | Masculino | Tom firme de narrador de documentário/livro |
| `pm_santa` | Português (`pt-BR`) | Masculino | Tom grave e acolhedor |
| `af_heart` | Inglês (`en-US`) | Feminino | Narradora literária premiada |
| `af_bella` | Inglês (`en-US`) | Feminino | Suave e natural |
| `am_adam` | Inglês (`en-US`) | Masculino | Narrador clássico em inglês |

---

## 3. Passo a Passo Manual: Piper TTS (Ultra-Rápido em C++ / ONNX)

O **Piper TTS** é ideal se você quiser converter um livro de 400 páginas em poucos minutos usando apenas CPU.

### Passo 3.1 — Baixar o binário nativo do Piper
No Linux (`x86_64`):
```bash
mkdir -p bin models
curl -L https://github.com/rhasspy/piper/releases/download/2023.11.14-2/piper_linux_x86_64.tar.gz \
  -o /tmp/piper.tar.gz
tar -xzf /tmp/piper.tar.gz -C bin/
ln -sf "$(pwd)/bin/piper/piper" bin/piper-tts
```

### Passo 3.2 — Baixar os modelos de voz `.onnx` + `.onnx.json`
Cada voz do Piper precisa de **dois arquivos** na pasta `models/`: o arquivo `.onnx` e o arquivo de configuração `.onnx.json`:

```bash
# Voz Masculina Natural em Português (pt_BR-faber-medium)
curl -L https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0/pt/pt_BR/faber/medium/pt_BR-faber-medium.onnx \
  -o models/pt_BR-faber-medium.onnx
curl -L https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0/pt/pt_BR/faber/medium/pt_BR-faber-medium.onnx.json \
  -o models/pt_BR-faber-medium.onnx.json

# Voz Rápida em Português (pt_BR-edresson-low)
curl -L https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0/pt/pt_BR/edresson/low/pt_BR-edresson-low.onnx \
  -o models/pt_BR-edresson-low.onnx
curl -L https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0/pt/pt_BR/edresson/low/pt_BR-edresson-low.onnx.json \
  -o models/pt_BR-edresson-low.onnx.json
```

### Passo 3.3 — Testar o Piper no Open Audiobook Creator
```bash
cargo run -p oac-cli -- render \
  --input ./output/livro_exemplo.pdf \
  --engine piper \
  --voice pt_BR-faber-medium \
  --format m4b
```

---

## 4. Passo a Passo Manual: Coqui XTTS-v2 (Clonagem de Voz)

Use o **Coqui XTTS-v2** quando quiser clonar uma voz a partir de um arquivo `.wav` de 6 a 15 segundos:

```bash
# Instalar o fork mantido da comunidade (coqui-tts) dentro do .venv
python3 -m venv .venv
./.venv/bin/pip install coqui-tts

# Testar verificação no CLI
cargo run -p oac-cli -- voices
```

---

## 5. Verificando a Instalação

A qualquer momento, execute o comando de diagnóstico:
```bash
cargo run -p oac-cli -- voices
```
Ele exibirá quais binários (`Piper`, `Kokoro`, `Coqui XTTS`, `FFmpeg`) e quais modelos `.onnx` estão prontos para uso!

