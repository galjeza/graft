use std::path::PathBuf;

use git2::{BranchType, Repository};

pub struct Git {
    repo: Repository,
}

const BASE_BRANCH: &str = "main";

impl Git {
    pub fn new(path: &str) -> Self {
        let repo = Repository::open(path).unwrap();
        Git { repo }
    }

    pub fn worktrees(&self) -> Vec<String> {
        let worktrees = self.repo.worktrees().unwrap();
        worktrees.iter().map(|w| w.unwrap().to_string()).collect()
    }

    pub fn branches(&self) -> Vec<String> {
        let branches = self.repo.branches(Some(BranchType::Local)).unwrap();
        return branches
            .map(|b| {
                let (branch, _branch_type) = b.unwrap();
                branch.name().unwrap().unwrap().to_string()
            })
            .collect();
    }

    pub fn create_branch(&self, branch_name: &str) {
        let base = self
            .repo
            .find_branch(BASE_BRANCH, BranchType::Local)
            .unwrap();
        let base_head = base.get().peel_to_commit().unwrap();
        self.repo.branch(branch_name, &base_head, false).unwrap();
    }

    pub fn ensure_branch(&self, branch_name: &str) {
        let branch_exists = self
            .repo
            .find_branch(branch_name, BranchType::Local)
            .is_ok();
        if !branch_exists {
            self.create_branch(branch_name);
        }
    }

    pub fn create_worktree(&self, worktree_name: &str) {
        let worktree_path = PathBuf::from(format!("./{}", worktree_name));
        self.repo
            .worktree(worktree_name, &worktree_path, None)
            .unwrap();
    }

    pub fn ensure_worktree(&self, worktree_name: &str) {
        if !self.repo.find_worktree(worktree_name).is_ok() {
            self.create_worktree(worktree_name);
        }
    }

    pub fn current_branch(&self) -> String {
        todo!()
    }

    pub fn delete_branch() -> () {
        todo!()
    }
}
