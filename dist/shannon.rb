class Shannon < Formula
  desc "Poly-shell built on nushell with seamless bash compatibility"
  homepage "https://github.com/ryanxcharles/shannon"
  url "https://github.com/ryanxcharles/shannon/releases/download/v0.5.6/shannon-0.5.6.tar.gz"
  sha256 "8d649109f001d1013115e5376c4a9c399d7b3338c7bb7137c7b6d8d54b160d27"
  license "MIT"

  depends_on "rust" => :build
  depends_on :macos

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    version_output = shell_output("#{bin}/shannon --version")
    assert_match "0.5.6", version_output
    assert_match "nushell 0.113.1", version_output

    assert_equal "3", shell_output("#{bin}/shannon -c '1 + 2'").strip
  end
end
