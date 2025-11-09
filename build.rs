fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_prost_build::configure()
        .protoc_arg("--experimental_allow_proto3_optional")
        .btree_map(".")
        .type_attribute(
            "common.Node",
            "#[derive(serde::Serialize, serde::Deserialize)]",
        )
        .type_attribute(
            "common.Membership",
            "#[derive(serde::Serialize, serde::Deserialize)]",
        )
        .type_attribute(
            "common.NodeIdSet",
            "#[derive(serde::Serialize, serde::Deserialize)]",
        )
        .type_attribute(
            "internal.LeaderId",
            "#[derive(serde::Serialize, serde::Deserialize, PartialOrd)]",
        )
        .type_attribute(
            "internal.Vote",
            "#[derive(serde::Serialize, serde::Deserialize)]",
        )
        .type_attribute(
            "internal.Entry",
            "#[derive(serde::Serialize, serde::Deserialize)]",
        )
        .compile_protos(
            &[
                "proto/common.proto",
                "proto/internal.proto",
                "proto/controller.proto",
            ],
            &["proto"],
        )?;

    Ok(())
}
