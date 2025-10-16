fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_prost_build::configure()
        .protoc_arg("--experimental_allow_proto3_optional")
        .btree_map(".")
        .type_attribute(
            "openraftpb.LeaderId",
            "#[derive(serde::Serialize, serde::Deserialize, PartialOrd)]",
        )
        .type_attribute(
            "openraftpb.Vote",
            "#[derive(serde::Serialize, serde::Deserialize)]",
        )
        .type_attribute(
            "openraftpb.Node",
            "#[derive(serde::Serialize, serde::Deserialize)]",
        )
        .type_attribute(
            "openraftpb.Membership",
            "#[derive(serde::Serialize, serde::Deserialize)]",
        )
        .type_attribute(
            "openraftpb.NodeIdSet",
            "#[derive(serde::Serialize, serde::Deserialize)]",
        )
        .type_attribute(
            "openraftpb.Entry",
            "#[derive(serde::Serialize, serde::Deserialize)]",
        )
        .compile_protos(&["proto/raft.proto", "proto/app_types.proto"], &["proto"])?;

    Ok(())
}
