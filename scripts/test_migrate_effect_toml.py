import json
import tempfile
import textwrap
import unittest
from pathlib import Path

from migrate_effect_toml import load_map, migrate_text

ROOT = Path(__file__).resolve().parent


class MigrateEffectToml(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.mapping = load_map(ROOT / "effect_type_map.json")

    def test_map_covers_deal_damage(self):
        self.assertEqual(
            self.mapping["deal_damage"],
            {"family": "damage", "mode": "target"},
        )

    def test_rewrites_flat_effect_table(self):
        src = textwrap.dedent(
            """\
            [[abilities.effects]]
            type = "deal_damage"
            amount = 3
            target = "any"
            """
        )
        out = migrate_text(src, self.mapping)
        self.assertIn('type = "damage"', out)
        self.assertIn('mode = "target"', out)
        self.assertNotIn('type = "deal_damage"', out)
        self.assertIn("amount = 3", out)

    def test_leaves_structural_sequence_type(self):
        src = textwrap.dedent(
            """\
            [[abilities.effects]]
            type = "sequence"

            [[abilities.effects.steps]]
            type = "draw_cards"
            count = 1
            """
        )
        out = migrate_text(src, self.mapping)
        self.assertIn('type = "sequence"', out)
        self.assertIn('type = "draw"', out)
        self.assertIn('mode = "cards"', out)

    def test_map_has_at_least_230_entries(self):
        self.assertGreaterEqual(len(self.mapping), 230)


if __name__ == "__main__":
    unittest.main()
