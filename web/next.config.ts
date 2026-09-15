import type { NextConfig } from "next";

const config: NextConfig = {
  // Ships the server plus only the dependencies it actually reached, rather
  // than all of node_modules. The Docker image is the reason.
  output: "standalone",
};

export default config;
