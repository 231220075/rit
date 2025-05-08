pub const OBJ_META: [&'static str ; 3] = ["blob", "tree", "commit"];

pub trait ObjType {
    const VALUE: usize;
}

pub struct Blob;
impl ObjType for Blob {
    const VALUE: usize = 0;
}

pub struct Tree;
impl ObjType for Tree {
    const VALUE: usize = 1;
}

pub struct Commit;
impl ObjType for Commit {
    const VALUE: usize = 1;
}
