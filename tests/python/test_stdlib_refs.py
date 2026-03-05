"""Tests for Python stdlib reference files in refs/std/."""
import os
import sys
import unittest

# Add project root to path
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..', '..', 'python'))

from polyref_py.ref_parser import parse_reference_file


REFS_STD_DIR = os.path.join(os.path.dirname(__file__), '..', '..', 'refs', 'std')


def load_ref(filename):
    """Load and parse a reference file from refs/std/."""
    path = os.path.join(REFS_STD_DIR, filename)
    with open(path, 'r') as f:
        content = f.read()
    ref_file = parse_reference_file(content)
    return ref_file.entries


class TestPathlibRef(unittest.TestCase):
    def test_pathlib_parses(self):
        entries = load_ref('pathlib.polyref')
        self.assertTrue(len(entries) > 0, "pathlib ref should produce entries")

    def test_pathlib_has_path_class_methods(self):
        entries = load_ref('pathlib.polyref')
        names = [e.name for e in entries]
        self.assertIn('exists', names)
        self.assertIn('mkdir', names)
        self.assertIn('read_text', names)
        self.assertIn('glob', names)


class TestOsRef(unittest.TestCase):
    def test_os_parses(self):
        entries = load_ref('os.polyref')
        self.assertTrue(len(entries) > 0, "os ref should produce entries")

    def test_os_has_key_functions(self):
        entries = load_ref('os.polyref')
        names = [e.name for e in entries]
        self.assertIn('getcwd', names)
        self.assertIn('listdir', names)
        self.assertIn('makedirs', names)
        self.assertIn('getenv', names)


class TestJsonRef(unittest.TestCase):
    def test_json_parses(self):
        entries = load_ref('json.polyref')
        self.assertTrue(len(entries) > 0, "json ref should produce entries")

    def test_json_has_key_functions(self):
        entries = load_ref('json.polyref')
        names = [e.name for e in entries]
        self.assertIn('dumps', names)
        self.assertIn('loads', names)
        self.assertIn('dump', names)
        self.assertIn('load', names)


class TestSubprocessRef(unittest.TestCase):
    def test_subprocess_parses(self):
        entries = load_ref('subprocess.polyref')
        self.assertTrue(len(entries) > 0, "subprocess ref should produce entries")

    def test_subprocess_has_key_functions(self):
        entries = load_ref('subprocess.polyref')
        names = [e.name for e in entries]
        self.assertIn('run', names)
        self.assertIn('check_output', names)


class TestDatetimeRef(unittest.TestCase):
    def test_datetime_parses(self):
        entries = load_ref('datetime.polyref')
        self.assertTrue(len(entries) > 0, "datetime ref should produce entries")

    def test_datetime_has_key_methods(self):
        entries = load_ref('datetime.polyref')
        names = [e.name for e in entries]
        self.assertIn('now', names)
        self.assertIn('strftime', names)
        self.assertIn('isoformat', names)


class TestTypingRef(unittest.TestCase):
    def test_typing_parses(self):
        entries = load_ref('typing.polyref')
        self.assertTrue(len(entries) > 0, "typing ref should produce entries")

    def test_typing_has_key_elements(self):
        entries = load_ref('typing.polyref')
        names = [e.name for e in entries]
        self.assertIn('cast', names)
        self.assertIn('get_type_hints', names)


if __name__ == '__main__':
    unittest.main()
