pub trait ObjType {
    const VALUE: &'static str;
}

pub struct Blob;
impl ObjType for Blob {
    const VALUE: &'static str = "blob";
}

pub struct Tree;
impl ObjType for Tree {
    const VALUE: &'static str = "tree";
}

pub struct Commit;
impl ObjType for Commit {
    const VALUE: &'static str = "commit";
}
