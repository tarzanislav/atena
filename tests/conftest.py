import os
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

import pytest

TEST_DB = os.environ.get("ATENA_TEST_DATABASE_URL")


@pytest.fixture(scope="session")
def db_url():
    if not TEST_DB:
        pytest.skip("ATENA_TEST_DATABASE_URL nie je nastavené – databázové testy sa preskočia")
    return TEST_DB


@pytest.fixture()
def conn(db_url, monkeypatch):
    monkeypatch.setenv("ATENA_DATABASE_URL", db_url)
    from atena import config
    monkeypatch.setattr(config.settings, "database_url", db_url)
    from atena.db import connect, init_schema
    c = connect(db_url)
    c.execute("DROP SCHEMA public CASCADE; CREATE SCHEMA public;")
    c.commit()
    init_schema(c)
    yield c
    c.close()


class Site:
    """Malý testovací web: robots.txt, RSS, stránky, PDF, menené odpovede."""

    def __init__(self):
        self.pages = {}
        self.hits = []

    def set(self, path, body, ctype="text/html; charset=utf-8", status=200, headers=None):
        self.pages[path] = (status, ctype, body.encode() if isinstance(body, str) else body, headers or {})


@pytest.fixture()
def site():
    s = Site()

    class H(BaseHTTPRequestHandler):
        def log_message(self, *a):
            pass

        def do_GET(self):
            s.hits.append(self.path)
            if self.path not in s.pages:
                self.send_response(404)
                self.end_headers()
                return
            status, ctype, body, headers = s.pages[self.path]
            etag = '"' + str(hash(body)) + '"'
            if self.headers.get("If-None-Match") == etag and status == 200:
                self.send_response(304)
                self.end_headers()
                return
            self.send_response(status)
            self.send_header("Content-Type", ctype)
            self.send_header("Content-Length", str(len(body)))
            self.send_header("ETag", etag)
            for k, v in headers.items():
                self.send_header(k, v)
            self.end_headers()
            self.wfile.write(body)

    srv = ThreadingHTTPServer(("127.0.0.1", 0), H)
    s.base = f"http://127.0.0.1:{srv.server_address[1]}"
    t = threading.Thread(target=srv.serve_forever, daemon=True)
    t.start()
    yield s
    srv.shutdown()
