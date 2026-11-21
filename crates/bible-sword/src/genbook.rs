//! General books (RawGenBook): a tree of sections such as
//! "/Book I/Chapter 1", each with its own text.

use crate::conf::decode;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Node {
    pub name: String,
    /// "/Book I/Chapter 1"
    pub path: String,
    pub depth: usize,
    pub children: Vec<usize>,
    pub parent: Option<usize>,
    offset: u32,
    size: u32,
}

#[derive(Debug)]
pub struct GenBook {
    prefix: PathBuf,
    /// nodes in reading order (depth-first); index 0 is the unnamed root
    pub nodes: Vec<Node>,
}

fn i32le(b: &[u8], at: usize) -> Option<i32> {
    Some(i32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

impl GenBook {
    pub fn open(prefix: &Path) -> std::io::Result<GenBook> {
        let p = prefix.display().to_string();
        let idx = fs::read(format!("{p}.idx"))?;
        let dat = fs::read(format!("{p}.dat"))?;
        // raw nodes keyed by their byte position in .idx
        struct Raw {
            // the file's parent link; the tree is rebuilt by walking child/next instead
            _parent: i32,
            next: i32,
            child: i32,
            name: String,
            offset: u32,
            size: u32,
        }
        let mut raw = std::collections::HashMap::new();
        for (i, c) in idx.chunks_exact(4).enumerate() {
            let at = u32::from_le_bytes(c.try_into().unwrap()) as usize;
            let (Some(parent), Some(next), Some(child)) =
                (i32le(&dat, at), i32le(&dat, at + 4), i32le(&dat, at + 8))
            else {
                continue;
            };
            let nstart = at + 12;
            let Some(nlen) = dat
                .get(nstart..)
                .and_then(|s| s.iter().position(|&b| b == 0))
            else {
                continue;
            };
            let name = decode(&dat[nstart..nstart + nlen]);
            let dpos = nstart + nlen + 1;
            let dsize = dat
                .get(dpos..dpos + 2)
                .map(|b| u16::from_le_bytes([b[0], b[1]]) as usize)
                .unwrap_or(0);
            let (offset, size) = if dsize >= 8 {
                let d = &dat[dpos + 2..dpos + 2 + 8];
                (
                    u32::from_le_bytes(d[0..4].try_into().unwrap()),
                    u32::from_le_bytes(d[4..8].try_into().unwrap()),
                )
            } else {
                (0, 0)
            };
            raw.insert(
                (i * 4) as i32,
                Raw {
                    _parent: parent,
                    next,
                    child,
                    name,
                    offset,
                    size,
                },
            );
        }
        // walk the tree depth-first from the root (idx position 0)
        let mut nodes: Vec<Node> = Vec::new();
        fn walk(
            raw: &std::collections::HashMap<i32, Raw>,
            at: i32,
            parent: Option<usize>,
            depth: usize,
            path: &str,
            nodes: &mut Vec<Node>,
            guard: &mut usize,
        ) {
            let mut cur = at;
            while cur >= 0 {
                *guard += 1;
                if *guard > 200_000 {
                    return;
                }
                let Some(r) = raw.get(&cur) else { return };
                let my_path = if r.name.is_empty() {
                    String::new()
                } else {
                    format!("{path}/{}", r.name)
                };
                let id = nodes.len();
                nodes.push(Node {
                    name: r.name.clone(),
                    path: my_path.clone(),
                    depth,
                    children: vec![],
                    parent,
                    offset: r.offset,
                    size: r.size,
                });
                if let Some(p) = parent {
                    nodes[p].children.push(id);
                }
                if r.child >= 0 {
                    walk(raw, r.child, Some(id), depth + 1, &my_path, nodes, guard);
                }
                if parent.is_none() {
                    return; // the root has no siblings
                }
                cur = r.next;
            }
        }
        let mut guard = 0;
        walk(&raw, 0, None, 0, "", &mut nodes, &mut guard);
        Ok(GenBook {
            prefix: PathBuf::from(p),
            nodes,
        })
    }

    pub fn text(&self, node: usize) -> Option<String> {
        let n = self.nodes.get(node)?;
        if n.size == 0 {
            return None;
        }
        let bdt = fs::read(format!("{}.bdt", self.prefix.display())).ok()?;
        let t = bdt.get(n.offset as usize..(n.offset + n.size) as usize)?;
        let s = decode(t);
        let s = s.trim_end_matches(['\0', '\n', '\r']).to_string();
        if s.trim().is_empty() {
            None
        } else {
            Some(s)
        }
    }

    pub fn find(&self, path: &str) -> Option<usize> {
        self.nodes.iter().position(|n| n.path == path)
    }

    /// Next / previous node that has text, for reading straight through.
    pub fn next_with_text(&self, from: usize, forward: bool) -> Option<usize> {
        let mut i = from as isize;
        loop {
            i += if forward { 1 } else { -1 };
            if i < 1 || i as usize >= self.nodes.len() {
                return None;
            }
            if self.nodes[i as usize].size > 0 {
                return Some(i as usize);
            }
        }
    }
}
