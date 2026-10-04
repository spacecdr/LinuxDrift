import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location('registration', Path(__file__).resolve().parents[1] / 'packaging/register-xscreensaver.py')
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)

class RegistrationTests(unittest.TestCase):
    def test_preserves_order_selection_and_is_idempotent(self):
        original = 'mode: one\nselected: 1\nprograms: \\\n  first --root \\n\\\n  second --root\nother: unchanged\n'
        result = module.register(original)
        self.assertIn('selected: 1', result)
        self.assertIn('other: unchanged', result)
        self.assertLess(result.index('second'), result.index('linuxdrift'))
        self.assertEqual(result, module.register(result))
        self.assertIn('second --root \\n\\\n', result)
    def test_new_user_inherits_system_hacks(self):
        result = module.register('', '*programs: \\\n  first --root \\n\\\n  second --root\n*other: no\n')
        self.assertIn('first --root', result)
        self.assertIn('second --root', result)
        self.assertIn('linuxdrift --root', result)
        self.assertNotIn('*other', result)
    def test_migrates_pre_release_name_in_place(self):
        original = 'selected: 0\nprograms: "LinuxFlux" linuxflux --root\n'
        result = module.register(original)
        self.assertIn('"LinuxDrift" linuxdrift --root', result)
        self.assertNotIn('LinuxFlux', result)
        self.assertEqual(result.count('linuxdrift --root'), 1)

    def test_empty_defaults(self):
        result = module.register('mode: random\n')
        self.assertIn('linuxdrift --root', result)
        self.assertEqual(result, module.register(result))

if __name__ == '__main__':
    unittest.main()
