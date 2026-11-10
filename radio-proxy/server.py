"""Radio proxy: turns live streams Lavalink cannot decode (HLS with raw AAC
segments, TV HLS, Kick live streams) into a plain MP3 HTTP stream.

GET /kafaradyo.mp3     -> Kafa Radyo
GET /cnnturk.mp3       -> CNN Turk Radyo
GET /trthaber.mp3      -> TRT Haber Radyo
GET /haberturk.mp3     -> Haberturk TV (audio only, 480p variant)
GET /kick/<slug>.mp3   -> audio of a live Kick channel (404 if offline)
"""
import http.server
import json
import re
import socketserver
import subprocess
import urllib.parse
import urllib.request

UA = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130 Safari/537.36"

STATIONS = {
    "kafaradyo": "https://moondigitaledge.radyotvonline.net/kafaradyo/playlist.m3u8",
    "cnnturk": "https://moondigitaledge2.radyotvonline.net/cnnturk/playlist.m3u8",
    "trthaber": "https://trt.radyotvonline.net/trthaber",
    "haberturk": "https://ciner.daioncdn.net/haberturktv/haberturktv_480p.m3u8?app=c98ab0b0-50cc-495b-bb37-778e91f5ff5b&ce=3",
    "damarfm": "https://live.radyositesihazir.com:10997/stream",
    # Music (URLs from radio-browser.info, highest-voted entries)
    "lofi": "http://streams.fluxfm.de/Chillhop/mp3-320",
    "paradise": "http://stream-uk1.radioparadise.com/aac-320",
    "rock": "http://stream.radioparadise.com/rock-320",
    "mellow": "http://stream.radioparadise.com/mellow-320",
    "groove": "https://ice2.somafm.com/groovesalad-128-mp3",
    "kexp": "https://kexp-mp3-128.streamguys1.com/kexp128.mp3",
    "jazz": "https://knkx-live-a.edge.audiocdn.com/6285_256k",
    "swisspop": "http://stream.srg-ssr.ch/srgssr/rsp/mp3/128",
}


def http_get(url: str) -> str:
    req = urllib.request.Request(url, headers={"User-Agent": UA, "Accept": "*/*"})
    with urllib.request.urlopen(req, timeout=10) as r:
        return r.read().decode("utf-8", "replace")


def lowest_variant(master_url: str) -> str:
    """Pick the lowest-bandwidth variant of an HLS master playlist (audio is all we need)."""
    text = http_get(master_url)
    best = None
    lines = text.splitlines()
    for i, line in enumerate(lines):
        if line.startswith("#EXT-X-STREAM-INF"):
            m = re.search(r"BANDWIDTH=(\d+)", line)
            bw = int(m.group(1)) if m else 1 << 62
            uri = lines[i + 1].strip() if i + 1 < len(lines) else ""
            if uri and (best is None or bw < best[0]):
                best = (bw, urllib.parse.urljoin(master_url, uri))
    return best[1] if best else master_url


def kick_url(slug: str):
    """Returns the HLS URL of a live Kick channel, or None if offline/unknown."""
    if not re.fullmatch(r"[A-Za-z0-9_\-]{1,40}", slug):
        return None
    try:
        data = json.loads(http_get(f"https://kick.com/api/v2/channels/{slug}"))
    except Exception as e:
        print(f"[radio-proxy] kick lookup failed for {slug}: {e}", flush=True)
        return None
    url = data.get("playback_url")
    if not url or not data.get("livestream"):
        return None
    return lowest_variant(url)


def resolve(path: str):
    key = path.split("?")[0].strip("/")
    if key.endswith(".mp3"):
        key = key[:-4]
    if key.startswith("kick/"):
        return kick_url(key[5:])
    return STATIONS.get(key)


# 4 seconds of MP3 silence (covers ffmpeg start-up), same format as the live output.
SILENCE = subprocess.run(
    [
        "ffmpeg", "-loglevel", "error",
        "-f", "lavfi", "-i", "anullsrc=r=48000:cl=stereo", "-t", "4",
        "-c:a", "libmp3lame", "-b:a", "128k",
        "-id3v2_version", "0", "-write_xing", "0",
        "-f", "mp3", "-",
    ],
    stdout=subprocess.PIPE,
    check=True,
).stdout


def ffmpeg_cmd(url: str) -> list:
    cmd = ["ffmpeg", "-loglevel", "warning", "-user_agent", UA,
           # Read at real-time speed so output is a steady flow, not bursts.
           # Bursts made Lavalink time out and reconnect -> audio gaps.
           "-re", "-probesize", "32768", "-analyzeduration", "500000"]
    if ".m3u8" in url:
        # Start 3 segments behind live: a cushion against slow segments.
        cmd += ["-live_start_index", "-3"]
    else:
        cmd += ["-reconnect", "1", "-reconnect_streamed", "1", "-reconnect_delay_max", "5"]
    cmd += [
        "-i", url,
        "-vn", "-ac", "2", "-ar", "48000",
        "-c:a", "libmp3lame", "-b:a", "128k",
        "-flush_packets", "1",
        "-id3v2_version", "0", "-write_xing", "0",
        "-f", "mp3", "-",
    ]
    return cmd


class Handler(http.server.BaseHTTPRequestHandler):
    def _headers(self):
        self.send_response(200)
        self.send_header("Content-Type", "audio/mpeg")
        self.send_header("Cache-Control", "no-cache")
        self.end_headers()

    def do_HEAD(self):
        if not resolve(self.path):
            self.send_error(404)
            return
        self._headers()

    def do_GET(self):
        url = resolve(self.path)
        if not url:
            self.send_error(404)
            return
        self._headers()
        proc = subprocess.Popen(ffmpeg_cmd(url), stdout=subprocess.PIPE, bufsize=0)
        try:
            # Lavalink's format detection times out if the first audio is slow.
            self.wfile.write(SILENCE)
            self.wfile.flush()
            while True:
                chunk = proc.stdout.read(4096)
                if not chunk:
                    break
                self.wfile.write(chunk)
                self.wfile.flush()
        except (BrokenPipeError, ConnectionResetError):
            pass
        finally:
            proc.kill()

    def log_message(self, fmt, *args):
        print("[radio-proxy] " + (fmt % args), flush=True)


class Server(socketserver.ThreadingMixIn, http.server.HTTPServer):
    daemon_threads = True


if __name__ == "__main__":
    print("[radio-proxy] listening on :8000", flush=True)
    Server(("", 8000), Handler).serve_forever()
