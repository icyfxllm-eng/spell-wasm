"""CC-AUDIO: the Azure DNS retry, and the things it must NOT retry.

The retry exists because this Mac's resolver drops out in bursts — six times
in three hours during the Swahili warm on 2026-10-03, each under two minutes,
each self-healing. What makes it safe is what it refuses to retry: an HTTP
error is a real answer from Azure, and retrying a 401 or a 429 is worse than
failing once.
"""

import os
import socket
import tempfile
import unittest
import urllib.error
from unittest import mock

# app.py refuses to import without a key; synthesis is never reached here.
os.environ.setdefault("GOOGLE_TTS_API_KEY", "test-not-a-key")
# db.init() runs at import and defaults to /data, which is read-only here.
os.environ.setdefault(
    "CLIMB_DB_PATH", os.path.join(tempfile.mkdtemp(), "climb.db")
)

import app  # noqa: E402


def gaierror():
    return urllib.error.URLError(socket.gaierror(8, "nodename nor servname provided"))


class DnsRetryTest(unittest.TestCase):
    def setUp(self):
        # The real delays are 2s and 5s; nobody needs to wait for them.
        patcher = mock.patch("app.time.sleep")
        self.sleep = patcher.start()
        self.addCleanup(patcher.stop)

    def _run(self, side_effect):
        with mock.patch("app.urllib.request.urlopen", side_effect=side_effect) as u:
            try:
                return app._urlopen_retrying_dns(mock.Mock(), 15, "test"), None, u
            except Exception as e:  # noqa: BLE001 - the test inspects it
                return None, e, u

    def test_a_single_dns_blip_is_absorbed(self):
        ok = mock.Mock()
        ok.read.return_value = b"audio"
        got, err, u = self._run([gaierror(), ok])
        self.assertIsNone(err)
        self.assertEqual(got, b"audio")
        self.assertEqual(u.call_count, 2)

    def test_it_survives_a_two_minute_burst(self):
        ok = mock.Mock()
        ok.read.return_value = b"audio"
        got, err, u = self._run([gaierror(), gaierror(), ok])
        self.assertIsNone(err)
        self.assertEqual(got, b"audio")
        self.assertEqual(u.call_count, 3)

    def test_a_resolver_that_never_comes_back_still_fails(self):
        # Three tries, then give up. A hang would be worse than an error.
        got, err, u = self._run([gaierror(), gaierror(), gaierror()])
        self.assertIsInstance(err, urllib.error.URLError)
        self.assertEqual(u.call_count, 3)

    def test_an_http_error_is_NOT_retried(self):
        # HTTPError subclasses URLError, which is exactly the trap. A 401 is a
        # bad key and a 429 is a rate limit; both are real answers.
        for code in (401, 429, 500):
            with self.subTest(code=code):
                err_obj = urllib.error.HTTPError(
                    "https://x", code, "nope", hdrs=None, fp=None
                )
                got, err, u = self._run([err_obj])
                self.assertIsInstance(err, urllib.error.HTTPError)
                self.assertEqual(err.code, code)
                self.assertEqual(u.call_count, 1, "an HTTP answer must not be retried")

    def test_a_non_dns_network_error_is_NOT_retried(self):
        # Connection refused means something answered. Only name resolution
        # failure is the transient this is for.
        refused = urllib.error.URLError(ConnectionRefusedError(61, "refused"))
        got, err, u = self._run([refused])
        self.assertIsInstance(err, urllib.error.URLError)
        self.assertEqual(u.call_count, 1)

    def test_it_backs_off_rather_than_spinning(self):
        ok = mock.Mock()
        ok.read.return_value = b"audio"
        self._run([gaierror(), gaierror(), ok])
        # First attempt immediate, then 2s, then 5s — covers a burst without
        # turning a real outage into a long hang.
        waited = [c.args[0] for c in self.sleep.call_args_list if c.args and c.args[0]]
        self.assertEqual(waited, [2.0, 5.0])


if __name__ == "__main__":
    unittest.main()
