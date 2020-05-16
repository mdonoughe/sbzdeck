fn main() {
    let mut res = winres::WindowsResource::new();
    res.set_icon("categoryIcon.ico");
    res.compile().unwrap();
}
