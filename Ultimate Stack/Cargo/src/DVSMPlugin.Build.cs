using UnrealBuildTool;
using System.IO;

public class DVSMPlugin : ModuleRules
{
    public DVSMPlugin(ReadOnlyTargetRules Target) : base(Target)
    {
        PCHUsage = PCHUsageMode.UseExplicitOrSharedPCHs;

        PublicDependencyModuleNames.AddRange(new string[]
        {
            "Core",
            "CoreUObject",
            "Engine"
        });

        string ThirdPartyPath = Path.Combine(ModuleDirectory, "../ThirdParty/dvsm_core");

        // Include headers
        PublicIncludePaths.Add(Path.Combine(ThirdPartyPath, "include"));

        // Link import library (Windows)
        if (Target.Platform == UnrealTargetPlatform.Win64)
        {
            string LibPath = Path.Combine(ThirdPartyPath, "lib/Win64/dvsm_core.lib");
            PublicAdditionalLibraries.Add(LibPath);

            string DLLPath = Path.Combine(ThirdPartyPath, "bin/Win64/dvsm_core.dll");
            RuntimeDependencies.Add("$(BinaryOutputDir)/dvsm_core.dll", DLLPath);
        }

        // Linux support (optional future-proofing)
        if (Target.Platform == UnrealTargetPlatform.Linux)
        {
            string LibPath = Path.Combine(ThirdPartyPath, "lib/Linux/libdvsm_core.so");
            PublicAdditionalLibraries.Add(LibPath);
        }
    }
}
