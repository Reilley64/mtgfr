import textwrap
import unittest
from pathlib import Path

from migrate_effect_toml import (
    check_paths,
    find_unmigrated_types,
    load_map,
    migrate_text,
)

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

    def test_rewrites_then_table_form(self):
        src = textwrap.dedent(
            """\
            [[abilities.effects]]
            type = "conditional"

            [[abilities.effects.then]]
            type = "draw_cards"
            count = 1
            """
        )
        out = migrate_text(src, self.mapping)
        self.assertIn('type = "conditional"', out)
        self.assertIn('type = "draw"', out)
        self.assertIn('mode = "cards"', out)
        self.assertNotIn('type = "draw_cards"', out)

    def test_rewrites_inline_then_array(self):
        src = textwrap.dedent(
            """\
            [[abilities.effects]]
            type = "conditional"
            condition = { type = "won_clash" }
            then = [{ type = "draw_cards", count = 1 }]
            """
        )
        out = migrate_text(src, self.mapping)
        self.assertIn('type = "draw", mode = "cards", count = 1', out)
        self.assertNotIn('type = "draw_cards"', out)
        self.assertIn('condition = { type = "won_clash" }', out)

    def test_rewrites_multiline_inline_modes(self):
        src = textwrap.dedent(
            """\
            [[abilities.effects]]
            type = "choose_one"
            modes = [
                { type = "look_at_top", count = 7 },
                { type = "put_counters_each", count = 2 },
            ]
            """
        )
        out = migrate_text(src, self.mapping)
        self.assertIn('type = "dig", mode = "look_at_top"', out)
        self.assertIn('type = "counters", mode = "put_counters_each"', out)
        self.assertNotIn('type = "look_at_top"', out)
        self.assertNotIn('type = "put_counters_each"', out)

    def test_find_unmigrated_detects_inline_then(self):
        src = textwrap.dedent(
            """\
            [[abilities.effects]]
            type = "conditional"
            then = [{ type = "deal_damage", amount = 3 }]
            """
        )
        hits = find_unmigrated_types(src, self.mapping)
        self.assertEqual(len(hits), 1)
        self.assertEqual(hits[0][1], "deal_damage")

    def test_check_paths_reports_unmigrated(self):
        src = textwrap.dedent(
            """\
            [[abilities.effects]]
            type = "deal_damage"
            """
        )
        path = ROOT / "_test_unmigrated.toml"
        path.write_text(src, encoding="utf-8")
        try:
            failures = check_paths([path], self.mapping)
            self.assertEqual(len(failures), 1)
            self.assertEqual(failures[0][1][0][1], "deal_damage")
        finally:
            path.unlink(missing_ok=True)

        migrated = migrate_text(src, self.mapping)
        path = ROOT / "_test_migrated.toml"
        path.write_text(migrated, encoding="utf-8")
        try:
            failures = check_paths([path], self.mapping)
            self.assertEqual(failures, [])
        finally:
            path.unlink(missing_ok=True)

    def test_migrate_is_idempotent(self):
        src = textwrap.dedent(
            """\
            [[abilities.effects]]
            type = "deal_damage"
            amount = 3

            [[abilities.effects]]
            type = "conditional"
            then = [{ type = "mill", amount = 3 }]
            """
        )
        once = migrate_text(src, self.mapping)
        twice = migrate_text(once, self.mapping)
        self.assertEqual(once, twice)
        self.assertEqual(once.count('mode = "target"'), 1)
        self.assertEqual(once.count('mode = "mill"'), 1)

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
