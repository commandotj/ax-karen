use std::collections::HashMap;
use std::path::{Path, PathBuf};

use monitor_disk::removal::{self, Plan, Target};
use monitor_disk::tree::Node;

#[derive(Debug, Default)]
pub struct DiskMarks {
    volumes: HashMap<PathBuf, Vec<Target>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MarkError {
    RootCannotBeMarked,
    PathOutsideScan,
    PathNotFound,
    CoveredBy(PathBuf),
}

impl DiskMarks {
    pub fn items(&self, root: &Path) -> &[Target] {
        self.volumes.get(root).map_or(&[], Vec::as_slice)
    }

    pub fn contains(&self, root: &Path, path: &Path) -> bool {
        self.items(root).iter().any(|item| item.path == path)
    }

    pub fn toggle(
        &mut self,
        root: &Path,
        tree: &Node,
        path: &Path,
    ) -> Result<bool, MarkError> {
        if path == root {
            return Err(MarkError::RootCannotBeMarked);
        }
        let relative = path
            .strip_prefix(root)
            .map_err(|_| MarkError::PathOutsideScan)?;
        if relative.as_os_str().is_empty() {
            return Err(MarkError::RootCannotBeMarked);
        }
        let node =
            node_at_relative(tree, relative).ok_or(MarkError::PathNotFound)?;
        let marks = self.volumes.entry(root.to_path_buf()).or_default();
        if let Some(index) = marks.iter().position(|item| item.path == path) {
            marks.remove(index);
            return Ok(false);
        }
        if let Some(ancestor) =
            marks.iter().find(|item| path.starts_with(&item.path))
        {
            return Err(MarkError::CoveredBy(ancestor.path.clone()));
        }

        marks.retain(|item| !item.path.starts_with(path));
        marks.push(Target {
            path: path.to_path_buf(),
            bytes: node.bytes,
            is_dir: node.is_dir(),
            hidden: is_hidden(path),
        });
        Ok(true)
    }

    pub fn unmark(&mut self, root: &Path, path: &Path) {
        if let Some(marks) = self.volumes.get_mut(root) {
            marks.retain(|item| item.path != path);
        }
    }

    pub fn clear(&mut self, root: &Path) {
        self.volumes.remove(root);
    }

    pub fn plan(&self, root: &Path) -> Plan {
        removal::plan(self.items(root), root)
    }

    pub fn refresh(&mut self, root: &Path, tree: &Node) {
        let Some(marks) = self.volumes.get_mut(root) else {
            return;
        };
        marks.retain_mut(|item| {
            let Some(relative) = item.path.strip_prefix(root).ok() else {
                return false;
            };
            let Some(node) = node_at_relative(tree, relative) else {
                return false;
            };
            item.bytes = node.bytes;
            item.is_dir = node.is_dir();
            true
        });
    }
}

fn node_at_relative<'a>(tree: &'a Node, path: &Path) -> Option<&'a Node> {
    path.components().try_fold(tree, |node, component| {
        let name = component.as_os_str().to_string_lossy();
        node.children
            .iter()
            .find(|child| child.name.as_ref() == name)
    })
}

fn is_hidden(path: &Path) -> bool {
    path.file_name()
        .is_some_and(|name| name.to_string_lossy().starts_with('.'))
}

#[cfg(test)]
mod tests {
    use super::{DiskMarks, MarkError};
    use monitor_disk::tree::{Metric, Node, NodeKind, aggregate};
    use std::path::{Path, PathBuf};

    fn tree(note_bytes: u64, extra: bool) -> Node {
        let mut root = Node::directory("home");
        let mut cache = Node::directory(".cache");
        cache.children.push(Node::entry("blob", NodeKind::File, 90));
        root.children.push(cache);
        root.children
            .push(Node::entry("note", NodeKind::File, note_bytes));
        if extra {
            root.children.push(Node::entry("extra", NodeKind::File, 5));
        }
        aggregate(&mut root, Metric::Bytes);
        root
    }

    #[test]
    fn mark_toggle_and_parent_absorption_keep_absolute_paths() {
        let root_path = Path::new("/home/user");
        let tree = tree(10, false);
        let mut marks = DiskMarks::default();
        let cache = Path::new("/home/user/.cache");
        let blob = Path::new("/home/user/.cache/blob");

        assert!(marks.toggle(root_path, &tree, blob).unwrap());
        assert_eq!(marks.items(root_path)[0].bytes, 90);
        assert!(marks.toggle(root_path, &tree, cache).unwrap());
        assert_eq!(marks.items(root_path).len(), 1);
        assert_eq!(marks.items(root_path)[0].path, cache);
        assert_eq!(
            marks.toggle(root_path, &tree, blob),
            Err(MarkError::CoveredBy(cache.to_path_buf()))
        );
        assert!(!marks.toggle(root_path, &tree, cache).unwrap());
    }

    #[test]
    fn root_and_outside_paths_cannot_be_marked() {
        let root_path = Path::new("/home/user");
        let tree = tree(10, false);
        let mut marks = DiskMarks::default();

        assert_eq!(
            marks.toggle(root_path, &tree, root_path),
            Err(MarkError::RootCannotBeMarked)
        );
        assert_eq!(
            marks.toggle(root_path, &tree, Path::new("/etc/passwd")),
            Err(MarkError::PathOutsideScan)
        );
    }

    #[test]
    fn rescan_refreshes_sizes_and_drops_missing_marks() {
        let root_path = Path::new("/home/user");
        let old_tree = tree(10, true);
        let new_tree = tree(20, false);
        let mut marks = DiskMarks::default();
        marks
            .toggle(root_path, &old_tree, Path::new("/home/user/note"))
            .unwrap();
        marks
            .toggle(root_path, &old_tree, Path::new("/home/user/extra"))
            .unwrap();

        marks.refresh(root_path, &new_tree);

        assert_eq!(marks.items(root_path).len(), 1);
        assert_eq!(marks.items(root_path)[0].bytes, 20);
    }

    #[test]
    fn roots_keep_marks_independent() {
        let first = PathBuf::from("/Volumes/one");
        let second = PathBuf::from("/Volumes/two");
        let tree = tree(10, false);
        let path = Path::new("/Volumes/one/note");
        let mut marks = DiskMarks::default();

        marks.toggle(&first, &tree, path).unwrap();

        assert!(marks.contains(&first, path));
        assert_eq!(marks.items(&second), []);
    }
}
