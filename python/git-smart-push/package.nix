{
  lib,
  python313Packages,
  git,
  runCommand,
}:

let
  package = python313Packages.buildPythonApplication {
    pname = "git-smart-push";
    version = (builtins.fromTOML (builtins.readFile ./pyproject.toml)).project.version;
    pyproject = true;
    src = lib.fileset.toSource {
      root = ./.;
      fileset = lib.fileset.unions [
        ./main.py
        ./pyproject.toml
        ./README.md
        (lib.fileset.fileFilter (file: file.hasExt "py") ./tests)
      ];
    };

    build-system = [ python313Packages.setuptools ];
    # Git is required even when the invoking shell has no Git on PATH.
    makeWrapperArgs = [ "--prefix PATH : ${lib.makeBinPath [ git ]}" ];
    pythonImportsCheck = [ "main" ];
    doCheck = true;
    checkPhase = ''
      runHook preCheck
      python -m unittest discover -s tests -v
      runHook postCheck
    '';

    passthru.tests.runtime =
      runCommand "git-smart-push-runtime"
        {
          nativeBuildInputs = [ git ];
        }
        ''
          export HOME="$TMPDIR/home"
          mkdir -p "$HOME"
          git init --bare remote.git
          git init work
          cd work
          git -c user.name=Test -c user.email=test@example.invalid commit --allow-empty -m initial
          git remote add origin ../remote.git
          PATH=/no-ambient-commands ${package}/bin/git-smart-push origin HEAD:refs/heads/main
          test "$(git rev-parse HEAD)" = "$(git --git-dir=../remote.git rev-parse main)"
          touch "$out"
        '';

    meta = {
      description = "Push with Git and optionally open the pull request creation URL";
      mainProgram = "git-smart-push";
      platforms = [
        "aarch64-darwin"
        "x86_64-linux"
      ];
    };
  };
in
package
