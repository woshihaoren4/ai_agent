 echo "start compile application..."

 python_rt_build_image_pull="docker pull wdshihaoren/python_rt:build-1.75-240527"
 echo "$python_rt_build_image_pull"
 ${python_rt_build_image_pull}

 python_rt_311_image_pull="docker pull python:3.11-alpine"
 echo "python_rt_311_image_pull"
 ${python_rt_311_image_pull}


#  if [ ! -e ".cargo/config.toml" ] ; then
#    echo "mkdir .cargo;touch .cargo/config.toml"
#    mkdir .cargo;touch .cargo/config.toml
#  fi
#  cargo_config_content='
#[target.x86_64-unknown-linux-musl]
#linker = "x86_64-linux-musl-gcc"'
#  echo "$cargo_config_content" > .cargo/config.toml
#  echo -e "write cargo config file :\n<---$cargo_config_content\n<---"

  echo "mv src/lib.rs src/lib_back.rs"
  mv src/lib.rs src/lib_back.rs
  echo "mv src/main_back.rs src/main.rs"
  mv src/main_back.rs src/main.rs
  echo "cd ../"
  cd ../

#  echo "clean bin file:rm ../target/x86_64-unknown-linux-musl/release/python_rt"
#  rm ../target/x86_64-unknown-linux-musl/release/python_rt

#  compile_cmd="cargo build --release --target=x86_64-unknown-linux-musl"
#  echo $compile_cmd
#  ${compile_cmd}
#  echo "compile success"

#  echo "rm -rf .cargo"
#  rm -rf .cargo
#  echo "cargo config file clear success."

  echo "start build image use docker..."
  if [ -n "$2" ]; then
      version=$2
  else
    timestamp=$(date +%s%N)
    version=${timestamp:2:8}
  fi
  tag="wdshihaoren/python_rt:$version"
  build_cmd="docker build -f python_rt/Dockerfile -t $tag ./"
  echo $build_cmd
  ${build_cmd}

  echo "cd python_rt"
  cd python_rt
  echo "mv src/lib_back.rs src/lib.rs"
  mv src/lib_back.rs src/lib.rs
  echo "mv src/main.rs src/main_back.rs"
  mv src/main.rs src/main_back.rs


  echo "docker build success image:$tag"
  docker_push_cmd="docker push $tag"
  echo $docker_push_cmd
  ${docker_push_cmd}

  echo "docker push success"