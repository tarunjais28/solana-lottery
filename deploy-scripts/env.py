import os
import subprocess as sp
import lib.env


def repo_relpath(*p):
    return os.path.join(os.path.dirname(__file__), "..", *p)


REPO_ROOT = repo_relpath(".")
ENV_TEMPLATE = ".env.example"


def main():
    files = get_env_files(REPO_ROOT)

    if ENV_TEMPLATE in files:
        files.remove(ENV_TEMPLATE)
    else:
        print(f"{ENV_TEMPLATE} not found")
        exit(-1)

    envs_template_path = repo_relpath(ENV_TEMPLATE)
    with open(envs_template_path) as f:
        envs_template, errors = lib.env.parse_file(f)

    if errors:
        print(f"{ENV_TEMPLATE}: ")
        print_errors(errors)
        exit(-1)

    envs_template_keys = set(envs_template.keys())
    envs_used_in_code = get_envs_used_in_code()
    envs_not_defined = envs_used_in_code - envs_template_keys

    envs_unused = envs_template_keys - envs_used_in_code

    if envs_unused:
        print(f"{ENV_TEMPLATE}: Unused envs")
        for v in envs_unused:
            print("", v)

    if envs_not_defined:
        print(f"{ENV_TEMPLATE}: Missing envs")
        for v in envs_not_defined:
            print("", v)

    if not envs_unused and not envs_not_defined:
        print(f"{ENV_TEMPLATE}: OK")

    for file in files:
        file_path = repo_relpath(file)
        with open(file_path) as f:
            envs, errors = lib.env.parse_file(f)
        if errors:
            print(f"{file}: Error while parsing")
            print_errors(errors)
            continue

        envs_keys = set(envs.keys())
        undeclared_keys = envs_keys - envs_template_keys
        if undeclared_keys:
            print(f"{file}: Error: Keys not declared in the template found")
            for key in undeclared_keys:
                print(f"  {key}")

    return 0


def print_errors(errors):
    for line_no, error in errors.items():
        print(f" {line_no}: {error}")


def get_envs_used_in_code():
    matchers = [
        'env( "$" )',
        'var( "$" )',
        'envconfig( from = "$"',
    ]

    matchers_regexp_list = []
    replacement_regexp_list = []
    for i, m in enumerate(matchers):
        r = m
        r = r.replace("(", r"\(")
        r = r.replace(")", r"\)")
        r = r.replace("$", r"(\w+)")
        r = r.replace(" ", r"\s*")
        matchers_regexp_list.append(r)
        replacement_regexp_list.append(f"${i+1}")

    matchers_regexp = "|".join(matchers_regexp_list)
    replacement_regexp = "".join(replacement_regexp_list)

    res = sp.run(
        [
            "rg",
            "-o",
            "--multiline",
            "-L",
            "-N",
            "--no-heading",
            "--no-filename",
            matchers_regexp,
            "-r",
            replacement_regexp,
        ],
        capture_output=True,
        cwd=REPO_ROOT,
    )
    return set(res.stdout.decode("utf8").splitlines())


def get_env_files(root):
    files = os.listdir(root)
    return [f for f in files if f == ".env" or f.startswith(".env.")]


def get_unused_envs(envs):
    unused_envs = []
    for env in envs:
        res = sp.run(
            ["rg", "--type", "rust", "-q", "-w", env, "workspace", "program"],
            cwd=REPO_ROOT,
        )
        if res.returncode != 0:
            unused_envs.append(env)
    return unused_envs


if __name__ == "__main__":
    main()
