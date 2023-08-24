const { Binary } = require("binary-install");
const os = require("os");

function getPlatform() {
  const type = os.type();
  const arch = os.arch();

  switch (`${type} ${arch}`) {
    case "Windows_NT x64":
      return "x86_64-pc-windows-gnu";
    case "Linux x64":
      return "x86_64-unknown-linux-gnu";
    case "Darwin x64":
      return "x86_64-apple-darwin";
  }

  throw new Error(`Unsupported platform: ${type} ${arch}`);
}

const { version, repository, name } = require("../package.json");

function getBinary() {
  const platform = getPlatform();
  const url = `${repository.url}/releases/download/v${version}/${name}_${platform}.tar.gz`;
  return new Binary(name, url, version);
}

module.exports = getBinary;
