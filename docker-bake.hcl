variable "IMAGE" {
  description = "Image repository for the built images."
  default     = "yeetypete/bootc-jetson"
}

variable "VERSION" {
  description = "Version tag for the built images."
  default     = "v0.0.0"
}

variable "REVISION" {
  description = "Git commit SHA for image labels."
  default     = ""
}

group "default" {
  targets = ["jetson-orin"]
}

target "_common" {
  pull = true
  labels = {
    "org.opencontainers.image.version"  = trimprefix(VERSION, "v")
    "org.opencontainers.image.revision" = REVISION
  }
  output = [
    "type=docker,compression=zstd",
    "type=oci,dest=image.oci,compression=zstd",
  ]
}

target "jetson-orin" {
  inherits   = ["_common"]
  context    = "./orin"
  dockerfile = "Dockerfile"
  platforms  = ["linux/arm64"]
  tags = [
    "${IMAGE}:orin-jp7.2",
    "${IMAGE}:orin-jp7.2-${trimprefix(VERSION, "v")}",
  ]
}
