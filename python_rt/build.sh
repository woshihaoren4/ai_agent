buildrs='
fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .out_dir("src/proto")
        .compile(
            &["proto/python_runtime_service.proto"],
            &["proto/"],
        )?;
    Ok(())
}
'
build_dependencies='
[build-dependencies]
tonic-build = { version = "0.11.0", features = ["prost"] }'

echo "create build.rs >>\n$buildrs\n"
echo "$buildrs" > build.rs

echo "append build_dependencies >> Cargo.toml\n$build_dependencies\n"
echo "$build_dependencies" >> Cargo.toml

clean_proto="rm src/proto/proto.rs"
echo "$clean_proto"
${clean_proto}

build_cmd="cargo test tests::build"
echo "$build_cmd"
${build_cmd}

rm_build_file_dependencies="sed -i '' -e :a -e '$d;N;2,3ba' -e 'P;D' Cargo.toml"
echo "$rm_build_file_dependencies"
sed -i '' -e :a -e '$d;N;2,3ba' -e 'P;D' Cargo.toml

rm_build_file="rm build.rs"
echo "$rm_build_file"
${rm_build_file}

echo "build proto over"