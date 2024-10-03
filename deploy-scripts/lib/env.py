def parse_file(lines):
    env = {}
    errors = {}

    line_no = 0

    for i, line in enumerate(lines):
        line_no = i + 1
        line = line.strip()
        if len(line) == 0:
            continue

        if line.startswith("#"):
            continue

        var, err = parse_export(line, env)
        if var is None:
            errors[line_no] = err
            continue

        name = var["name"]
        value = var["value"]

        env[name] = value

    return env, errors


def parse_export(line: str, env: dict[str, str] = {}):
    if not line.startswith("export "):
        return None, f"Line doesn't start with export: {line}"

    line = line[len("export ") :]
    line = line.strip()

    name, *rest = line.split("=", 1)
    rest = rest[0]

    val = ""
    var = ""
    comment = ""

    in_quote = False
    in_var = False
    has_unquoted_space = False

    i = 0
    while True:
        if i < len(rest):
            c = rest[i]
        elif i == len(rest):
            c = ""  # sentinel value to trigger cleanup of in_var
        else:
            break

        if not in_var:
            if c == " " and not in_quote:
                has_unquoted_space = True
            elif c == "$":
                in_var = True
            elif c == '"':
                in_quote = not in_quote
            elif c == "#":
                comment = rest[i + 1 :].strip()
                break
            elif has_unquoted_space:
                return None, "Unquoted space"
            else:
                val += c
            i += 1
        else:
            if c.isalnum() or c == "_":
                var += c
                i += 1
            else:
                # no i += 1.
                # character is not advanced so that it can be consumed by the
                # next iteration.
                in_var = False
                if var in env:
                    var_res = env[var]
                    if not in_quote and " " in var_res:
                        return (
                            None,
                            f"Variable ${var}'s value ({var_res}) contains space, but is not quoted",
                        )
                    val += var_res
                    var = ""
                else:
                    return None, f"Unknown variable ${var}"
    return {"name": name, "value": val, "comment": comment}, None


# Tests

if __name__ == "__main__":
    import unittest

    class Tests(unittest.TestCase):
        def _test_ok(self, res_err, res_expected):
            res, err = res_err
            self.assertFalse(err, "err is not falsy")
            self.assertEqual(res, res_expected)

        def _test_err(self, res_err, msg):
            res, err = res_err
            self.assertIsNotNone(err, f"err was supposed to be: /{msg}/. res is {res}")
            self.assertRegex(err, msg)

        def _test_err_lines(self, res_err, line, msg):
            res, err = res_err
            self.assertIsNotNone(err, f"err was None. res is {res}")
            self.assertIsNotNone(err.get(line), f"err[{line}] was None. res is {res}")
            self.assertRegex(err.get(line), msg)

        def test_parse_export(self):
            self._test_ok(
                parse_export("export FOO=BAR # asd"),
                {"name": "FOO", "value": "BAR", "comment": "asd"},
            )
            self._test_ok(
                parse_export('export FOO="BAR BAZ""Foo Bar"'),
                {"name": "FOO", "value": "BAR BAZFoo Bar", "comment": ""},
            )
            self._test_ok(
                parse_export('export FOO=$X"$X $X""$X"', env={"X": "123"}),
                {"name": "FOO", "value": "123123 123123", "comment": ""},
            )
            self._test_ok(
                parse_export('export FOO="$X $Y"', env={"X": "123", "Y": "asd"}),
                {"name": "FOO", "value": "123 asd", "comment": ""},
            )
            self._test_ok(
                parse_export('export FOO="$X""$Y"', env={"X": "123", "Y": "asd"}),
                {"name": "FOO", "value": "123asd", "comment": ""},
            )

            self._test_err(parse_export("FOO=BAR"), "export")
            self._test_err(parse_export("export FOO=BAR BAZ"), "Unquoted space")
            self._test_err(parse_export("export FOO=$BAR"), "Unknown variable")

        def test_parse_file(self):
            self._test_ok(
                parse_file(["export FOO=BAR", "export BAR=$FOO"]),
                {"FOO": "BAR", "BAR": "BAR"},
            )
            self._test_err_lines(parse_file(["FOO=BAR"]), 1, "export")
            self._test_err_lines(
                parse_file(["export FOO=BAR", "export BAR=$BAZ"]), 2, "Unknown variable"
            )

    unittest.main()
