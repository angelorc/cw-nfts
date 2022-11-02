import codegen from "@cosmwasm/ts-codegen";

codegen({
  contracts: [
    {
      name: "Bs721Base",
      dir: "../contracts/bs721-base/schema",
    },
    {
      name: "Bs721Launchpad",
      dir: "../contracts/bs721-launchpad/schema",
    },
    /*{
      name: "Bs721Royalty",
      dir: "../contracts/bs721-royalty/schema",
    },*/
  ],
  outPath: "./src/",

  // options are completely optional ;)
  options: {
    bundle: {
      bundleFile: "index.ts",
      scope: "contracts",
    },
    types: {
      enabled: true,
    },
    client: {
      enabled: true,
    },
    reactQuery: {
      enabled: false,
      optionalClient: true,
      version: "v4",
      mutations: true,
      queryKeys: true,
      queryFactory: true,
    },
    recoil: {
      enabled: false,
    },
    messageComposer: {
      enabled: true,
    },
  },
}).then(() => {
  console.log("✨ all done!");
});