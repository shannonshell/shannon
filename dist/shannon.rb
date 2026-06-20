class Shannon < Formula
  desc "Poly-shell built on nushell with seamless bash compatibility"
  homepage "https://github.com/shannonshell/shannon"
  url "https://github.com/shannonshell/shannon/releases/download/v1.0.0/shannon-1.0.0.tar.gz"
  sha256 "94fae37468806eb764d9416350e4dbc641598fdaa57e69306084f0b62b1da5c0"
  license "MIT"

  depends_on "rust" => :build
  depends_on :macos

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    version_output = shell_output("#{bin}/shannon --version")
    assert_match "1.0.0", version_output
    assert_match "nushell 0.113.1", version_output

    assert_equal "3", shell_output("#{bin}/shannon -c '1 + 2'").strip
  end
end
