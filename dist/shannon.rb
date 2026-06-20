class Shannon < Formula
  desc "Poly-shell built on nushell with seamless bash compatibility"
  homepage "https://github.com/shannonshell/shannon"
  url "https://github.com/shannonshell/shannon/releases/download/v0.5.7/shannon-0.5.7.tar.gz"
  sha256 "9ee34faa76b8a60530f7360d172b1094f02a93e022a7d29decf635d90f9b995c"
  license "MIT"

  depends_on "rust" => :build
  depends_on :macos

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    version_output = shell_output("#{bin}/shannon --version")
    assert_match "0.5.7", version_output
    assert_match "nushell 0.113.1", version_output

    assert_equal "3", shell_output("#{bin}/shannon -c '1 + 2'").strip
  end
end
