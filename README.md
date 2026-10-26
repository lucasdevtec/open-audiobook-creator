# 🎧 Open Audiobook Creator

Sistema **open-source** multiplataforma (**Windows, Linux e macOS**) escrito em **Rust** (`oac-core`, `oac-cli`, `oac-desktop` Tauri v2) e **React + TypeScript** para converter livros em **PDF, EPUB e TXT** em audiobooks completos (**M4B com capítulos e capa**, **MP3** e **WAV**) utilizando motores de síntese de voz (TTS) locais e gratuitos.

---

## ✨ Funcionalidades Principais

- **Ingestão Multi-Formato (`PDF`, `EPUB`, `TXT`)**:
  - Extração de texto estruturado e metadados nativos.
  - Detecção automática de capítulos (`Capítulo 1`, `Chapter I`, `Prólogo`, headings HTML/Markdown).
- **Limpeza Inteligente de Páginas (`cleaner`)**:
  - Detecção estatística e remoção automática de cabeçalhos e rodapés repetidos nas margens das páginas.
  - Remoção de números de página isolados (`Página 12`, `14 / 200`, `- 15 -`).
  - Reconstrução de palavras hifenizadas na quebra de linha (`naturali-\ndade` → `naturalidade`).
  - Segmentação inteligente em sentenças respeitando limites de contexto dos modelos neurais.
- **Editor & Revisor Pré-Áudio**:
  - Revise e edite o texto limpo de cada capítulo, compare lado a lado com o texto bruto do PDF, mescle capítulos ou desative seções indesejadas (como bibliografia ou índice) antes de sintetizar.
- **Motores TTS Locais e Gratuitos (`tts`)**:
  - **Piper TTS** (Rápido e leve em CPU via ONNX — vozes `pt_BR-faber-medium`, `pt_BR-edresson-low`, `en_US-lessac-high`).
  - **Kokoro-82M ONNX** (Altíssima naturalidade — vozes `pf_dora`, `pm_alex`, `af_heart`).
  - **Coqui XTTS-v2** (Suporte a clonagem de voz e síntese multilíngue).
  - **Sintetizador Acústico Integrado em Rust** (Fallback offline zero-dependência para preview e validação imediata).
- **Exportação Profissional (`audio`)**:
  - **M4B**: Formato nativo de audiobook com tabela de capítulos (`chpl` / `FFMETADATA1`) e metadados.
  - **MP3**: Arquivo único ou separado por capítulo com tags ID3v2.
  - **Cache Inteligente por Trecho (SHA-256)**: Se você editar apenas um parágrafo de um capítulo, somente aquele trecho é re-sintetizado.

---

## 🗂️ Arquitetura do Repositório

```text
open-audiobook-creator/
├── Cargo.toml                  # Rust Cargo Workspace
├── crates/
│   ├── oac-core/               # Biblioteca principal (Parsers PDF/EPUB/TXT, Cleaner, TTS, Audio M4B/MP3)
│   ├── oac-cli/                # CLI (`oac`) + Servidor de Estúdio Local (`oac serve`)
│   └── oac-desktop/            # Aplicação Desktop Nativa (Tauri v2)
├── ui/                         # Interface Gráfica React + TypeScript + Tailwind CSS
└── scripts/                    # Scripts utilitários de setup e execução
```

---

## 📦 Instalação das Dependências e Motores TTS (Kokoro, Piper, XTTS e FFmpeg)

Consulte o guia completo em **[docs/INSTALACAO_DEPENDENCIAS.md](docs/INSTALACAO_DEPENDENCIAS.md)** para o passo a passo detalhado em **Linux, Windows e macOS**.

### Resumo Rápido (Linux / Ubuntu / Debian)

1. **Dependências do Sistema (FFmpeg, espeak-ng e WebKitGTK para Tauri)**:
   ```bash
   ./scripts/setup-linux-deps.sh
   ```

2. **Instalador Automático do Kokoro-82M ONNX + Piper TTS (Modelos `pt-BR` e `en-US`)**:
   ```bash
   chmod +x scripts/install-tts-models.sh
   ./scripts/install-tts-models.sh
   ```
   Esse script configura automaticamente:
   - **Kokoro-82M ONNX**: Baixa `models/kokoro-v1_0.onnx` e `models/voices-v1.0.bin` (incluindo as vozes brasileiras `pf_dora` e `pm_alex`), instala `kokoro-onnx` em `.venv` e cria o executável `bin/kokoro-tts`.
   - **Piper TTS**: Baixa o binário oficial em `bin/piper/` e as vozes `pt_BR-faber-medium.onnx`, `pt_BR-edresson-low.onnx` e `en_US-lessac-high.onnx` em `models/`.

3. **Verificar se todos os motores foram detectados**:
   ```bash
   cargo run -p oac-cli -- voices
   ```

---

## 🚀 Como Executar

### 1. Usando a Linha de Comando (`oac-cli`)

Compile e execute a demonstração completa (gera um PDF com múltiplos capítulos, cabeçalhos, números de página e hifenizações, limpa o texto e exporta o audiobook `.m4b`):

```bash
cargo run -p oac-cli -- demo
```

Outros comandos úteis:

```bash
# Extrair capítulos de um PDF/EPUB/TXT para um arquivo de projeto editável (.oac.json)
cargo run -p oac-cli -- extract --input meu_livro.pdf --output projeto.oac.json

# Listar diagnóstico de motores locais (Piper, Kokoro, XTTS, FFmpeg) e vozes disponíveis
cargo run -p oac-cli -- voices

# Renderizar um PDF, EPUB ou projeto .oac.json para M4B ou MP3
cargo run -p oac-cli -- render --input meu_livro.pdf --format m4b --engine piper --voice pt_BR-faber-medium
```

### 2. Usando a Interface Gráfica (Estúdio Visual)

Você pode rodar a interface gráfica de duas maneiras:

#### Opção A: Estúdio Local (Sem exigir pacotes WebKitGTK do sistema)
Em um terminal, inicie o backend Rust:
```bash
cargo run -p oac-cli -- serve --port 14200
```
Em outro terminal, inicie a interface React:
```bash
cd ui
npm install
npm run dev
```
Acesse `http://localhost:5173`.

#### Opção B: Janela Desktop Nativa (Tauri v2 — Windows, Linux e macOS)
No Linux (Ubuntu/Debian), instale antes as bibliotecas do WebKitGTK e FFmpeg:
```bash
./scripts/setup-linux-deps.sh
```
E execute o app desktop Tauri em `crates/oac-desktop`.

---

## 🧪 Testes Automatizados

```bash
cargo test --workspace
```

---

## 📄 Licença

Distribuído sob a licença **MIT**. Veja [LICENSE](LICENSE) para mais informações.

