#!/usr/bin/env python3

import pathlib
import subprocess as sp
from lib.commandline import Command
from lib.colors import red, green, bold
import os

envs = {
    "devnet": {
        "solana_env": "devnet",
        "k8s_env": "development",
        "admin": "admin.json",
        "payer": "admin.json",
    },
    "devnet-qk": {
        "solana_env": "https://autumn-virulent-emerald.solana-devnet.discover.quiknode.pro/ada85a6f973511ba3cd2ccf2bbb8d69c38a8c784/",
        "k8s_env": "development",
        "admin": "admin.json",
        "payer": "admin.json",
    },
    "testnet": {
        "solana_env": "testnet",
        "k8s_env": "testnet",
        "admin": "admin.json",
        "payer": "admin.json",
    },
    "testnet2": {
        "solana_env": "testnet",
        "k8s_env": "testnet2",
        "admin": "admin.json",
        "payer": "admin.json",
    },
    "mainnet-test": {
        "solana_env": "mainnet-beta",
        "k8s_env": None,
        "admin": "admin.json",
        "payer": "mainnet-test-payer.json",
    },
}

env_vars = os.environ.copy() | {"RUST_LOG": "error"}

relpath = lambda p: os.path.join(os.path.dirname(__file__), p)

K8S_DIR = relpath("../k8s")
PROGRAM_OUT_DIR = relpath("./program-out")

PROGRAM_NEZHA_STAKING = "nezha-staking"
PROGRAM_NEZHA_VRF = "nezha-vrf"
programs = {
    PROGRAM_NEZHA_STAKING: {
        "crate": "nezha_staking",
        "path": relpath("../program/nezha-staking"),
        "cli": {
            "crate": "nezha-staking-cli",
            "path": relpath("../program/nezha-staking-cli"),
        },
    },
    PROGRAM_NEZHA_VRF: {
        "crate": "nezha_vrf",
        "path": relpath("../program/nezha-vrf"),
        "cli": {
            "crate": "nezha-vrf-cli",
            "path": relpath("../program/nezha-vrf-cli"),
        },
    },
}


def main():
    cmd = (
        Command("Nezha deployment script")
        .positional(
            "env",
            "Environment",
            list(envs.keys()),
        )
        .subcommand(
            "new-program-id",
            Command(
                "Generate a new program id, without updating config files or deploying"
            ).positional("program", "The program to deploy", list(programs.keys())),
        )
        .subcommand(
            "deploy",
            Command("Deploy the program")
            .positional("program", "The program to deploy", list(programs.keys()))
            .switch(
                ["--new", "-n"],
                "new",
                "Deploy to a new program id, update the config files",
                "bool",
                default=False,
            )
            .switch(
                ["--no-init"],
                "no_init",
                "Don't run init instructions",
                "bool",
                default=False,
            ),
        )
        .subcommand(
            "update-keys",
            Command("Update the keys in k8s and .env files")
            .positional("program", "The program to deploy", list(programs.keys()))
            .switch(
                ["--new", "-n"],
                "new",
                "Generate a new program id",
                "bool",
                default=False,
            ),
        )
        .subcommand(
            "init-program",
            Command("Initialize the program").positional(
                "program", "The program to deploy", list(programs.keys())
            ),
        )
        .subcommand(
            "program-id",
            Command("Show program id").positional(
                "program", "The program to deploy", list(programs.keys())
            ),
        )
        .subcommand(
            "admin-balance",
            Command("Show admin balance"),
        )
        .subcommand(
            "admin-close-buffers",
            Command("Close the buffer accounts of failed deploys and recover the SOLs"),
        )
        .subcommand(
            "payer-balance",
            Command("Show payer balance"),
        )
        .subcommand(
            "admin-airdrop",
            Command("Airdrop 1 SOL to admin"),
        )
    )
    out = cmd.parse()

    if out["command"] == "new-program-id":
        new_program_id(**out)
    elif out["command"] == "deploy":
        deploy(**out)
    elif out["command"] == "update-keys":
        update_keys(**out)
    elif out["command"] == "init-program":
        init_program(**out)
    elif out["command"] == "program-id":
        show_program_id(**out)
    elif out["command"] == "admin-balance":
        show_admin_balance(**out)
    elif out["command"] == "payer-balance":
        show_payer_balance(**out)
    elif out["command"] == "admin-airdrop":
        admin_airdrop(**out)
    elif out["command"] == "admin-close-buffers":
        admin_close_buffers(**out)
    else:
        print("Unrecognized command:", out["command"])


# Actions


def new_program_id(env, program, **kwargs):
    _new_key(env, program)


def deploy(env, program, new, no_init, **kwargs):
    if new:
        _new_key(env, program)
        _update_key(env, program)
    _deploy(env, program)
    if not no_init:
        _init_program(env, program)


def init_program(env, program, **kwargs):
    _init_program(env, program)


def update_keys(env, program, new, **kwargs):
    if new:
        _new_key(env, program)
    _update_key(env, program)


def show_program_id(env, program, **kwargs):
    program_key_file = _get_program_key_file(env, program)
    pubkey = _get_pubkey(program_key_file)
    print(pubkey)


def show_admin_balance(env, **kwargs):
    admin_keypair = envs[env]["admin"]
    solana_env = envs[env]["solana_env"]
    _show_balance(solana_env, admin_keypair)


def show_payer_balance(env, **kwargs):
    payer_keypair = envs[env]["payer"]
    solana_env = envs[env]["solana_env"]
    _show_balance(solana_env, payer_keypair)


def admin_airdrop(env, **kwargs):
    solana_env = envs[env]["solana_env"]
    admin_keypair = envs[env]["payer"]
    admin_pubkey = _get_pubkey(admin_keypair)

    sp_call(["solana", "-u", solana_env, "airdrop", "1", admin_pubkey])


def admin_close_buffers(env, **kwargs):
    solana_env = envs[env]["solana_env"]
    admin_keypair = envs[env]["payer"]

    sp_call(
        [
            "solana",
            "program",
            "close",
            "--buffers",
            "-u",
            solana_env,
            "-k",
            admin_keypair,
        ]
    )


# Helpers


def _show_balance(solana_env, keypair):
    sp_call(
        [
            "solana-keygen",
            "pubkey",
            keypair,
        ]
    )
    sp_call(
        ["solana", "-u", solana_env, "balance", "-k", keypair],
    )


def _new_key(env, program):
    sp_call(["mkdir", "-p", "program-keys"])
    sp_call(
        [
            "solana-keygen",
            "new",
            "-o",
            f"program-keys/{env}-{program}.json",
            "--no-bip39-passphrase",
            "--force",
        ]
    )


def _update_key(env, program):
    program_key_file = _get_program_key_file(env, program)
    program_id = _get_pubkey(program_key_file)

    k8s_env = envs[env].get("k8s_env", None)
    if k8s_env:
        if program == PROGRAM_NEZHA_STAKING:
            k8s_replacements = [
                (
                    f"{K8S_DIR}/lottery/{k8s_env}/deployment.yaml",
                    "SOLANA_STAKING_PROGRAM_ID",
                    program_id,
                ),
                (
                    f"{K8S_DIR}/indexer-deposits/{k8s_env}/deployment.yaml",
                    "INDEXER_DEPOSITS_SOLANA_PROGRAM_ID",
                    program_id,
                ),
                (
                    f"{K8S_DIR}/indexer-epochs/{k8s_env}/deployment.yaml",
                    "SOLANA_STAKING_PROGRAM_ID",
                    program_id,
                ),
            ]
        elif program == PROGRAM_NEZHA_VRF:
            k8s_replacements = []
        else:
            k8s_replacements = []

        for yaml_file, key, val in k8s_replacements:
            try:
                replace_env_k8s(yaml_file, key, val)
            except FileNotFoundError:
                print("File doesn't exist for this env, ignoring", yaml_file)
                pass
            print()

    if program == PROGRAM_NEZHA_STAKING:
        dotenv_replacements = [
            (f"../.env.{env}", "SOLANA_STAKING_PROGRAM_ID", program_id),
            (f"../.env.{env}", "DEMO_SOLANA_STAKING_PROGRAM_ID", program_id),
        ]
    elif program == PROGRAM_NEZHA_VRF:
        dotenv_replacements = [
            (f"../.env.{env}", "NEZHA_VRF_PROGRAM_ID", program_id),
        ]
    else:
        dotenv_replacements = []

    for env_file, key, val in dotenv_replacements:
        try:
            replace_env(env_file, key, val)
        except FileNotFoundError:
            print("File doesn't exist for this env, ignoring", env_file)
            pass
        print()


def _deploy(env, program):
    program_key_file = _get_program_key_file(env, program)

    solana_env = envs[env]["solana_env"]

    admin_key_file = envs[env]["admin"]
    if not pathlib.Path(admin_key_file).exists():
        print("Admin keypair doesn't exist:", admin_key_file)
        return

    payer_key_file = envs[env]["admin"]
    if not pathlib.Path(payer_key_file).exists():
        print("Admin keypair doesn't exist:", payer_key_file)
        return

    program_info = programs[program]
    program_crate = program_info["crate"]
    program_path = program_info["path"]

    print("Building: ", program_path)
    sp_call(
        [
            "cargo",
            "build-sbf",
            "--sbf-out-dir",
            PROGRAM_OUT_DIR,
            # "--",
            # "-p",
            # program_info["crate"],
        ],
        cwd=program_path,
    )
    sp_call(
        [
            "rm",
            # We are not using this. Deleting to avoid confusion.
            f"{PROGRAM_OUT_DIR}/{program_crate}-keypair.json",
        ]
    )
    print("Deploying to", solana_env)
    sp_call(
        [
            "solana",
            "program",
            "deploy",
            "-u",
            solana_env,
            f"{PROGRAM_OUT_DIR}/{program_crate}.so",
            "--program-id",
            program_key_file,
            "--upgrade-authority",
            admin_key_file,
            "-k",
            payer_key_file,
        ]
    )


def _init_program(env, program):
    cmds = [
        "init",
    ]
    if program == PROGRAM_NEZHA_STAKING and "mainnet" in env:
        cmds.append("francium-init")
    cli = programs[program]["cli"]

    for cmd in cmds:
        print(bold(cmd))
        sp_call(
            ["cargo", "run", "--", cmd],
            env_file=f"../.env.{env}",
            cwd=cli["path"],
        )


# Utils


def _get_program_key_file(env, program):
    program_key_file = f"program-keys/{env}-{program}.json"
    if not pathlib.Path(program_key_file).exists():
        print("Keypair doesn't exist. Please generate a new one.", program_key_file)
        exit(-1)
    return program_key_file


def _get_pubkey(key_file):
    program_id = sp.run(
        ["solana-keygen", "pubkey", key_file], capture_output=True, text=True
    ).stdout.strip()
    return program_id


# Config editing helpers


def replace_env(env_file, env, val):
    print(f"Updating {env_file}:")

    f = open(env_file, "r")
    lines = f.readlines()
    f.close()

    for i in range(len(lines)):
        line = lines[i]
        if env in line:
            x = line.index(env) + len(env)
            # skip ['=', ' ']
            while line[x] in "= ":
                x += 1
            line_new = line[:x] + val + "\n"
            if line == line_new:
                print("Already up-to-date")
                return

            print(red("- " + line.strip()))
            print(green("+ " + line_new.strip()))
            lines[i] = line_new

            break
    else:
        print(f"Can't find {env} in {env_file}")
        exit(-1)

    f = open(env_file, "w")
    f.writelines(lines)
    f.close()


def replace_env_k8s(yaml_file, env, val):
    f = open(yaml_file, "r")
    lines = f.readlines()
    f.close()

    print(f"Updating {yaml_file}:")

    for i in range(len(lines)):
        line = lines[i]
        if f"name: {env}" in line:
            line = lines[i + 1]
            x = line.index("value: ") + len("value: ")
            line_new = line[:x] + val + "\n"
            if line == line_new:
                print("Already up-to-date")
                return

            print(red("- " + line.strip()))
            print(green("+ " + line_new.strip()))
            lines[i + 1] = line_new

            break
    else:
        print(f"Can't find {env} in {yaml_file}")
        exit(-1)

    f = open(yaml_file, "w")
    f.writelines(lines)
    f.close()


# Subprocess helpers


def sp_call(*args, env_file=None, env=None, **kwargs):
    new_env = env_vars
    if env:
        new_env |= env

    if env_file:
        with open(env_file) as f:
            for line in f:
                original_line = line
                line = line.strip()

                if line == "":
                    continue
                if line.startswith("#"):
                    continue
                if line.startswith("export"):
                    line = line[len("export") :].strip()
                if "=" not in line:
                    print(f"Invalid line in {env_file}:")
                    print(original_line)
                    print("Can't find '='")
                    exit(-1)
                name, val = line.split("=")
                new_env[name] = val

    print("$", " ".join(args[0]))
    return sp.call(*args, **kwargs, env=new_env)


if __name__ == "__main__":
    main()
