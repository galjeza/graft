use anyhow::{Context, Result};
use git2::{BranchType, Repository, WorktreeAddOptions};
use std::{fs, path::PathBuf};

use crate::session;

const WORKTREE_DIR: &str = ".worktrees";

pub struct Repo {
    inner: Repository,
}

impl Repo {
    pub fn open() -> Result<Self> {
        let inner = Repository::discover(".").context("Not inside a Git repository")?;
        Ok(Self { inner })
    }

    pub fn root(&self) -> Result<PathBuf> {
        self.inner
            .workdir()
            .context("Repository has no working directory")
            .map(PathBuf::from)
    }

    // --------------------------------------------
    // Branch handling
    // --------------------------------------------

    pub fn delete_local_branch(&self, branch: &str) -> Result<()> {
        let mut b = self.inner.find_branch(branch, BranchType::Local)?;
        b.delete()?;
        Ok(())
    }

    // --------------------------------------------
    // Worktree open
    // --------------------------------------------

    pub fn open_worktree(&self, branch: &str, ephemeral: bool, delete_branch: bool) -> Result<()> {
        let path = self.ensure_worktree(branch)?;

        let status = session::launch(&path, &branch)?;

        if ephemeral {
            if let Err(e) = session::delete(branch) {
                eprintln!("Failed to delete session: {e}");
            }

            if let Err(e) = self.remove_worktree(branch) {
                eprintln!("Failed to remove worktree: {e}");
            }

            if delete_branch {
                if let Err(e) = self.delete_local_branch(branch) {
                    eprintln!("Failed to delete branch: {e}");
                }
            }
        }

        if !status.success() {
            anyhow::bail!("Zellij exited with non-zero status");
        }

        Ok(())
    }

    // --------------------------------------------
    // Worktree creation
    // --------------------------------------------

    pub fn ensure_worktree(&self, branch: &str) -> Result<PathBuf> {
        let base = self.root()?.join(WORKTREE_DIR);
        let path = base.join(branch);

        if path.exists() {
            return Ok(path);
        }

        fs::create_dir_all(&base)?;

        let opts = WorktreeAddOptions::new();
        self.inner
            .worktree(branch, &path, Some(&opts))
            .with_context(|| format!("Failed to create worktree '{branch}'"))?;

        Ok(path)
    }

    // --------------------------------------------
    // Remove branch + worktree
    // --------------------------------------------

    pub fn remove_branch(&self, branch: &str, delete_branch: bool) -> Result<()> {
        session::delete(branch)?;
        self.remove_worktree(branch)?;
        if delete_branch {
            self.delete_local_branch(branch)?;
        }

        Ok(())
    }

    fn remove_worktree(&self, branch: &str) -> Result<()> {
        let base = self.root()?.join(WORKTREE_DIR);
        let target_path = base.join(branch);

        if let Some(name) = self.inner.worktrees()?.iter().flatten().find(|name| {
            if let Ok(wt) = self.inner.find_worktree(name) {
                wt.path() == target_path
            } else {
                false
            }
        }) {
            self.inner.find_worktree(name)?.prune(None)?;
        }

        // Fallback: ensure directory removed
        if target_path.exists() {
            fs::remove_dir_all(&target_path)?;
        }

        Ok(())
    }

    // --------------------------------------------
    // List
    // --------------------------------------------

    pub fn list_worktrees(&self) -> Result<()> {
        for name in self.inner.worktrees()?.iter().flatten() {
            let wt = self.inner.find_worktree(name)?;
            println!("{name}  {}", wt.path().display());
        }
        Ok(())
    }
}
