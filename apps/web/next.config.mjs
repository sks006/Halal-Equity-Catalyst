import { createRequire } from "module";

const require = createRequire(import.meta.url);

/** @type {import('next').NextConfig} */
const nextConfig = {
  reactStrictMode: true,
  transpilePackages: ["@equity-catalyst/sdk", "sdk"],
  webpack: (config) => {
    config.resolve.alias = {
      ...config.resolve.alias,
      "@pythnetwork/pyth-lazer-sdk$": require.resolve("@pythnetwork/pyth-lazer-sdk"),
    };
    config.resolve.fallback = {
      ...config.resolve.fallback,
      fs: false,
      net: false,
      tls: false,
      crypto: false,
    };
    return config;
  },
};

export default nextConfig;
