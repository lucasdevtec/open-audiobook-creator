#!/usr/bin/env python3
"""
Ponte CLI de alta fidelidade para o motor Kokoro-82M ONNX.
Suporta:
- Blends de vetores de estilo vocal (Cronista, Bardo, Voz das Sombras, Professor, Contadora)
- Modos de Tom / Estilo de Narração (--style storyteller | professor | dramatic | natural)
- Worker local persistente via Unix Socket mantendo o modelo aquecido em memória (zero reload entre frases).
"""
import argparse
import json
import os
import re
import socket
import subprocess
import sys
import tempfile
import time

_UID = os.getuid() if hasattr(os, "getuid") else os.environ.get("USERNAME", "win")
SOCKET_PATH = os.path.join(tempfile.gettempdir(), f"oac_kokoro_worker_v3_{_UID}.sock")
WIN_TCP_PORT = 51423
IDLE_TIMEOUT_SECS = 300


# Blends literários customizados (todos falando Português do Brasil 'pt-br')
VOICE_BLENDS = {
    "blend_cronista": ("pt-br", [("pm_santa", 0.58), ("bm_george", 0.42)]),
    "blend_bardo": ("pt-br", [("pm_alex", 0.52), ("bm_fable", 0.48)]),
    "blend_sombrio": ("pt-br", [("am_onyx", 0.55), ("pm_santa", 0.45)]),
    "blend_professor": ("pt-br", [("pm_alex", 0.60), ("bm_daniel", 0.40)]),
    "blend_professora": ("pt-br", [("pf_dora", 0.62), ("bf_emma", 0.38)]),
    "blend_contadora": ("pt-br", [("pf_dora", 0.52), ("af_bella", 0.28), ("af_heart", 0.20)]),
}


def resolve_voice_and_lang(kokoro, voice_code: str):
    clean = voice_code.replace("kokoro-", "").strip()
    if clean in VOICE_BLENDS:
        lang, recipe = VOICE_BLENDS[clean]
        blended = None
        for v_name, weight in recipe:
            vec = kokoro.get_voice_style(v_name) * weight
            blended = vec if blended is None else (blended + vec)
        return blended, lang

    if clean.startswith(("pf_", "pm_")):
        return clean, "pt-br"
    if clean.startswith(("ef_", "em_")):
        return clean, "es"
    if clean.startswith(("ff_",)):
        return clean, "fr-fr"
    if clean.startswith(("bf_", "bm_")):
        return clean, "en-gb"
    if clean.startswith(("af_", "am_")):
        return clean, "en-us"
    return clean, "pt-br"


def create_kokoro_instance(model_path: str, voices_path: str):
    import onnxruntime as ort
    from kokoro_onnx import Kokoro

    ort.set_default_logger_severity(3)
    opts = ort.SessionOptions()
    opts.intra_op_num_threads = os.cpu_count() or 6
    opts.graph_optimization_level = ort.GraphOptimizationLevel.ORT_ENABLE_ALL

    # Nota: No Kokoro-v1_0.onnx, o operador ScatterND possui índices duplicados na matriz de alinhamento
    # que sofrem condição de corrida no ROCMExecutionProvider (gerando áudio corrompido).
    # Usamos CUDAExecutionProvider (quando disponível em NVIDIA) ou CPUExecutionProvider multi-thread
    # com worker persistente em RAM para garantir áudio 100% cristalino e rápido.
    available = ort.get_available_providers()
    if "CUDAExecutionProvider" in available:
        providers = ["CUDAExecutionProvider", "CPUExecutionProvider"]
    else:
        providers = ["CPUExecutionProvider"]

    sess = ort.InferenceSession(model_path, sess_options=opts, providers=providers)
    return Kokoro.from_session(sess, voices_path)


def synthesize_with_style(kokoro, text: str, voice_code: str, speed: float, style: str):
    import numpy as np

    voice_target, lang = resolve_voice_and_lang(kokoro, voice_code)

    # Ajustes prosódicos por estilo preservando o contexto completo da frase no Transformer do Kokoro
    style_cfg = {
        "storyteller": {
            "speed_mult": 0.94,
            "pause_para": 0.65,
        },
        "dramatic": {
            "speed_mult": 0.88,
            "pause_para": 0.85,
        },
        "professor": {
            "speed_mult": 0.96,
            "pause_para": 0.55,
        },
        "natural": {
            "speed_mult": 1.0,
            "pause_para": 0.45,
        },
    }.get(
        style,
        {
            "speed_mult": 0.94,
            "pause_para": 0.65,
        },
    )

    effective_speed = max(0.55, min(2.2, speed * style_cfg["speed_mult"]))
    sample_rate = 24000

    paragraphs = [p.strip() for p in re.split(r"\n\s*\n", text) if p.strip()]
    audio_segments = []

    for p_idx, para in enumerate(paragraphs):
        # Passa o parágrafo inteiro para o Kokoro manter a entonação natural contínua
        samples, sample_rate = kokoro.create(
            para,
            voice=voice_target,
            speed=effective_speed,
            lang=lang,
        )
        if len(samples) > 0:
            audio_segments.append(samples.astype(np.float32))

        if p_idx + 1 < len(paragraphs):
            para_pause = int(style_cfg["pause_para"] * sample_rate)
            audio_segments.append(np.zeros(para_pause, dtype=np.float32))

    if not audio_segments:
        return np.zeros(int(0.2 * sample_rate), dtype=np.float32), sample_rate

    full_audio = np.concatenate(audio_segments)
    return full_audio, sample_rate


def run_worker_daemon(model_path: str, voices_path: str) -> int:
    import soundfile as sf

    use_unix = hasattr(socket, "AF_UNIX") and os.name != "nt"
    if use_unix and os.path.exists(SOCKET_PATH):
        try:
            os.unlink(SOCKET_PATH)
        except OSError:
            pass

    kokoro = create_kokoro_instance(model_path, voices_path)
    if use_unix:
        srv = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        srv.bind(SOCKET_PATH)
    else:
        srv = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        srv.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        srv.bind(("127.0.0.1", WIN_TCP_PORT))
    srv.listen(8)
    srv.settimeout(IDLE_TIMEOUT_SECS)

    try:
        while True:
            try:
                conn, _ = srv.accept()
            except socket.timeout:
                break
            with conn:
                data = b""
                while True:
                    chunk = conn.recv(65536)
                    if not chunk:
                        break
                    data += chunk
                    if b"\n" in chunk:
                        break
                if not data:
                    continue
                req = json.loads(data.decode("utf-8").strip())
                try:
                    voice = req["voice"]
                    speed = float(req["speed"])
                    style = req.get("style", "storyteller")
                    text = req["text"]
                    output = req["output"]
                    samples, sample_rate = synthesize_with_style(
                        kokoro, text, voice, speed, style
                    )
                    sf.write(output, samples, sample_rate)
                    resp = {"ok": True, "providers": kokoro.sess.get_providers()}
                except Exception as exc:
                    resp = {"ok": False, "error": str(exc)}
                conn.sendall((json.dumps(resp) + "\n").encode("utf-8"))
    finally:
        srv.close()
        if use_unix and os.path.exists(SOCKET_PATH):
            try:
                os.unlink(SOCKET_PATH)
            except OSError:
                pass
    return 0


def request_via_worker(
    model_path: str,
    voices_path: str,
    text: str,
    voice: str,
    speed: float,
    style: str,
    output: str,
) -> bool:
    use_unix = hasattr(socket, "AF_UNIX") and os.name != "nt"

    payload = (
        json.dumps(
            {
                "text": text,
                "voice": voice,
                "speed": speed,
                "style": style,
                "output": os.path.abspath(output),
            }
        )
        + "\n"
    ).encode("utf-8")

    for attempt in range(2):
        if not use_unix or os.path.exists(SOCKET_PATH):
            try:
                cli = (
                    socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
                    if use_unix
                    else socket.socket(socket.AF_INET, socket.SOCK_STREAM)
                )
                with cli:
                    cli.settimeout(120.0)
                    if use_unix:
                        cli.connect(SOCKET_PATH)
                    else:
                        cli.connect(("127.0.0.1", WIN_TCP_PORT))
                    cli.sendall(payload)
                    resp_bytes = b""
                    while True:
                        buf = cli.recv(4096)
                        if not buf:
                            break
                        resp_bytes += buf
                        if b"\n" in buf:
                            break
                    resp = json.loads(resp_bytes.decode("utf-8").strip())
                    if resp.get("ok"):
                        return True
            except OSError:
                pass

        if attempt == 0:
            popen_kwargs = {
                "stdin": subprocess.DEVNULL,
                "stdout": subprocess.DEVNULL,
                "stderr": subprocess.DEVNULL,
                "start_new_session": True,
            }
            if os.name == "nt":
                popen_kwargs["creationflags"] = getattr(
                    subprocess, "CREATE_NO_WINDOW", 0x08000000
                )
            subprocess.Popen(
                [
                    sys.executable,
                    os.path.abspath(__file__),
                    "--daemon",
                    "--models-dir",
                    os.path.dirname(model_path),
                ],
                **popen_kwargs,
            )
            for _ in range(50):
                if use_unix and os.path.exists(SOCKET_PATH):
                    time.sleep(0.05)
                    break
                if not use_unix:
                    try:
                        with socket.create_connection(
                            ("127.0.0.1", WIN_TCP_PORT), timeout=0.1
                        ):
                            break
                    except OSError:
                        pass
                time.sleep(0.1)

    return False


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Kokoro-82M ONNX CLI Bridge for Open Audiobook Creator"
    )
    parser.add_argument(
        "--voice",
        default="blend_cronista",
        help="ID da voz ou blend (ex: blend_cronista, blend_bardo, pm_santa, pf_dora)",
    )
    parser.add_argument(
        "--speed", type=float, default=1.0, help="Velocidade da fala (ex: 1.0)"
    )
    parser.add_argument(
        "--style",
        default="storyteller",
        help="Tom/Estilo de narração: storyteller, professor, dramatic, natural",
    )
    parser.add_argument("--output", help="Caminho do arquivo WAV de saída")
    parser.add_argument(
        "--models-dir",
        default=None,
        help="Diretório contendo kokoro-v1_0.onnx e voices-v1.0.bin",
    )
    parser.add_argument(
        "--daemon",
        action="store_true",
        help="Executa em modo worker persistente mantendo o modelo em RAM",
    )
    args = parser.parse_args()

    root_dir = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
    models_dir = args.models_dir or os.path.join(root_dir, "models")
    model_path = os.path.join(models_dir, "kokoro-v1_0.onnx")
    voices_path = os.path.join(models_dir, "voices-v1.0.bin")

    if not os.path.exists(model_path):
        print(f"Modelo Kokoro não encontrado em: {model_path}", file=sys.stderr)
        return 2
    if not os.path.exists(voices_path):
        print(
            f"Arquivo de vozes Kokoro não encontrado em: {voices_path}", file=sys.stderr
        )
        return 2

    if args.daemon:
        return run_worker_daemon(model_path, voices_path)

    if not args.output:
        print("Parâmetro --output é obrigatório.", file=sys.stderr)
        return 1

    text = sys.stdin.read().strip()
    if not text:
        print("Nenhum texto recebido via stdin.", file=sys.stderr)
        return 1

    if request_via_worker(
        model_path,
        voices_path,
        text,
        args.voice,
        args.speed,
        args.style,
        args.output,
    ):
        return 0

    import soundfile as sf

    kokoro = create_kokoro_instance(model_path, voices_path)
    samples, sample_rate = synthesize_with_style(
        kokoro, text, args.voice, args.speed, args.style
    )
    sf.write(args.output, samples, sample_rate)
    return 0


if __name__ == "__main__":
    sys.exit(main())
