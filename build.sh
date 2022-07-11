#运行命令rustup target add aarch64-unknown-linux-gnu，添加aarch64-unknown-linux-gnu rust toolchain到系统
#修改cargo的config文件，配置新的目标架构// .cargo/config
#1
#2
#[target.aarch64-unknown-linux-gnu]
#linker = "aarch64-linux-gnu-gcc"
#最后cargo build时需要添加参数--target aarch64-unknown-linux-gnu 。若是想要省略该参数则需要对config作如下的修改，以改变默认的构建目标
#1
#2
#[build]
#target = "aarch64-unknown-linux-gnu"
#另外，也可以给build --target aarch64-unknown-linux-gnu 命令设置别名从而缩短构建命令。比如按下面的方式修改config文件后，就可以使用cargo build_aarch64来构建程序了
#1
#2
#[alias]
#build_aarch64 = "build --target aarch64-unknown-linux-gnu"
#至此，我们就得到了适用于aarch64-unknown-linux环境的二进制可执行目标文件
