use std::path::PathBuf;

use git2::{BranchType, Repository, Worktree, WorktreeAddOptions};

pub struct Git {
    repo: Repository,
}

const BASE_BRANCH: &str = "main";
const WORKTREE_DIR: &str = "./.worktrees";

impl Git {
    pub fn new(path: &str) -> Self {
        let repo = Repository::open(path).unwrap();
        Git { repo }
    }

    pub fn worktrees(&self) -> Vec<(String, PathBuf)> {
        let mut result = Vec::new();

        let names = self.repo.worktrees().unwrap();

        for name in names.iter().flatten() {
            if let Ok(wt) = self.repo.find_worktree(name) {
                result.push((name.to_string(), wt.path().to_path_buf()));
            }
        }

        result
    }

    pub fn create_worktree(&self, worktree_name: &str) -> Worktree {
        let mut options = WorktreeAddOptions::new();
        options.checkout_existing(true);

        let worktree_path = PathBuf::from(format!("{}/{}", WORKTREE_DIR, worktree_name));
        println!("Creating worktree at path: {:?}", worktree_path);

        self.repo
            .worktree(worktree_name, &worktree_path, Some(&options))
            .unwrap()
    }

    pub fn ensure_worktree(&self, worktree_name: &str) -> Worktree {
        match self.repo.find_worktree(worktree_name) {
            Ok(worktree) => worktree,
            Err(_) => self.create_worktree(worktree_name),
        }
    }

    pub fn delete_branch() -> () {
        todo!()
    }
}
