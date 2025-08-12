fn main() {
    // embed-resource v3 需要提供第二个参数（宏定义列表）；此处无需宏，传空数组即可
    let _ = embed_resource::compile("windows/app.rc", &[] as &[&str]);
}
