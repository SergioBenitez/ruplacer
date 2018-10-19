import argparse
import shutil
import subprocess
import mimetypes
import os

import github3
from path import Path


def run(*cmd, check=True, quiet=False):
    if not quiet:
        print("::", *cmd)
    if check:
        subprocess.check_call(cmd)
    else:
        return subprocess.call(cmd)


def copy(src, dest):
    print(src, "->", dest)
    Path(src).copy(dest)


def build_release():
    run("cargo", "build", "--release")


def populate_staging(staging_path, *, platform):
    if platform == "windows":
        ext = ".exe"
    else:
        ext = ""
    copy("target/release/ruplacer" + ext, staging_path)
    copy("README.md", staging_path)
    copy("CHANGELOG.md", staging_path)
    copy("LICENSE", staging_path)


def make_archive(archive_path, *, platform):
    if platform == "windows":
        archive_format = "zip"
    else:
        archive_format = "gztar"
    res = shutil.make_archive(
        archive_path, archive_format, root_dir=".", base_dir=archive_path
    )
    print(":: generated", res)
    return Path(res)


def generate_deb():
    rc = run("cargo", "deb", "--version", check=False, quiet=True)
    if rc != 0:
        print("cargo deb --version failed, installing cargo-deb")
        run("cargo", "install", "cargo-deb")
    cargo_out = subprocess.check_output(["cargo", "deb"]).decode().strip()
    return Path(cargo_out)


def upload_artifacts(artifact_paths, *, tag):
    github_token = os.environ["GITHUB_TOKEN"]

    gh = github3.GitHub()
    gh.login(token=github_token)

    repo = gh.repository("supertanker", "ruplacer")

    # We nee the release to exist before we can
    # upload assets, but may be it has
    # been created by an other job
    try:
        repo.create_release(tag, name=tag)
    except github3.GitHubError as github_error:
        print(github_error)

    release = repo.release_from_tag(tag)

    mimetypes.init()
    for artifact_path in artifact_paths:
        name = artifact_path.name
        (mimetype, _) = mimetypes.guess_type(name)
        print(":: Uploading asset", mimetype, name, artifact_path)
        release.upload_asset(mimetype, name, artifact_path)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--tag", required=True)
    parser.add_argument("--platform", required=True)
    args = parser.parse_args()
    tag = args.tag

    if not tag.startswith("v"):
        print("tag not starting with v, exiting")
        return

    version = tag[1:]
    platform = args.platform

    build_release()
    staging_path = Path("ruplacer-%s-%s" % (version, platform))
    staging_path.makedirs_p()

    populate_staging(staging_path, platform=platform)

    dist_path = Path("dist")
    dist_path.rmtree_p()
    dist_path.makedirs_p()

    artifacts = []
    archive_path = make_archive(staging_path, platform=platform)
    artifacts.append(archive_path)

    if "linux" in platform:
        deb = generate_deb()
        artifacts.append(deb)

    upload_artifacts(artifacts, tag=tag)


if __name__ == "__main__":
    main()
