{pkgs, ...}: {
  projectRootFile = "flake.nix";
  programs = {
    alejandra.enable = true;
    rustfmt.enable = true;
    ktfmt.enable = true;
    shfmt.enable = true;
    dockerfmt.enable = true;
  };
}
