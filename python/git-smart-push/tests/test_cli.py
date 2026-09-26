import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


SCRIPT = Path(__file__).resolve().parents[1] / "main.py"
URL = "https://github.com/example/project/pull/new/topic"


class PushTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.directory = Path(self.temp.name)
        self.env = {
            **os.environ,
            "PATH": str(self.directory),
            "GIT_OPEN_CMD": "",
            "TEST_DIRECTORY": str(self.directory),
            "TEST_GIT_EXIT": "0",
        }
        self.command("git", """
import json, os, sys
from pathlib import Path
Path(os.environ['TEST_DIRECTORY'], 'git-args').write_text(json.dumps(sys.argv[1:]))
print('push stdout')
print('remote: Create a pull request: """ + URL + """', file=sys.stderr)
sys.exit(int(os.environ['TEST_GIT_EXIT']))
""")
        self.command("browser with spaces", """
import os, sys
from pathlib import Path
Path(os.environ['TEST_DIRECTORY'], 'opened').write_text(sys.argv[1])
sys.exit(int(os.environ.get('TEST_OPEN_EXIT', '0')))
""")

    def command(self, name, body):
        path = self.directory / name
        path.write_text(f"#!{sys.executable}\n{body}")
        path.chmod(0o755)
        return path

    def run_push(self, answer="", *args):
        return subprocess.run(
            [sys.executable, str(SCRIPT), *args],
            input=answer, text=True, capture_output=True, env=self.env, check=False,
        )

    def test_no_opener_prints_url_without_prompt_and_forwards_arguments(self):
        import json

        result = self.run_push("", "--force-with-lease", "origin", "HEAD")
        self.assertEqual(result.returncode, 0)
        self.assertIn(URL, result.stdout)
        self.assertIn("push stdout", result.stdout)
        self.assertIn("remote: Create a pull request", result.stderr)
        self.assertNotIn("[Y/n]", result.stdout)
        self.assertEqual(
            json.loads((self.directory / "git-args").read_text()),
            ["push", "--force-with-lease", "origin", "HEAD"],
        )

    def test_missing_opener_does_not_prompt(self):
        self.env["GIT_OPEN_CMD"] = "/nonexistent/browser"
        result = self.run_push()
        self.assertEqual(result.returncode, 0)
        self.assertIn(URL, result.stdout)
        self.assertNotIn("[Y/n]", result.stdout)

    def test_configured_opener_receives_url_as_single_argument(self):
        self.env["GIT_OPEN_CMD"] = str(self.directory / "browser with spaces")
        result = self.run_push("y\n")
        self.assertEqual(result.returncode, 0)
        self.assertIn("[Y/n]", result.stdout)
        self.assertEqual((self.directory / "opened").read_text(), URL)

    def test_declining_does_not_open(self):
        self.env["GIT_OPEN_CMD"] = "browser with spaces"
        result = self.run_push("n\n")
        self.assertEqual(result.returncode, 0)
        self.assertFalse((self.directory / "opened").exists())

    def test_opener_failure_preserves_git_exit_status(self):
        self.env.update(GIT_OPEN_CMD="browser with spaces", TEST_OPEN_EXIT="7", TEST_GIT_EXIT="3")
        result = self.run_push("y\n")
        self.assertEqual(result.returncode, 3)
        self.assertIn("Failed to open URL", result.stderr)

    def test_no_url_does_not_prompt(self):
        self.command("git", "print('Everything up-to-date')")
        self.env["GIT_OPEN_CMD"] = "browser with spaces"
        result = self.run_push()
        self.assertEqual(result.returncode, 0)
        self.assertNotIn("[Y/n]", result.stdout)


if __name__ == "__main__":
    unittest.main()
