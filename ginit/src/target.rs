use crate::util;
use into_result::command::CommandResult;
use std::{collections::BTreeMap, fmt};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Profile {
    Debug,
    Release,
}

impl Profile {
    pub fn is_debug(self) -> bool {
        self == Profile::Debug
    }

    pub fn is_release(self) -> bool {
        self == Profile::Release
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Profile::Debug => "debug",
            Profile::Release => "release",
        }
    }
}

pub trait TargetTrait<'a>: Sized {
    const DEFAULT_KEY: &'static str;

    fn all() -> &'a BTreeMap<&'a str, Self>;

    fn default_ref() -> &'a Self {
        Self::all()
            .get(Self::DEFAULT_KEY)
            .expect("Developer error: no target matched `DEFAULT_KEY`")
    }

    fn for_name(name: &str) -> Option<&'a Self> {
        Self::all().get(name)
    }

    fn for_arch(arch: &str) -> Option<&'a Self> {
        Self::all().values().find(|target| target.arch() == arch)
    }

    fn triple(&'a self) -> &'a str;
    fn arch(&'a self) -> &'a str;

    fn rustup_add(&'a self) -> CommandResult<()> {
        util::rustup_add(self.triple())
    }
}

#[derive(Debug)]
pub struct TargetInvalid<'a> {
    name: String,
    possible: Vec<&'a str>,
}

impl fmt::Display for TargetInvalid<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Target {:?} is invalid; the possible targets are {:?}",
            self.name, self.possible,
        )
    }
}

pub fn get_targets<'a, Iter, I, T, U>(
    targets: Iter,
    // we use `dyn` so the type doesn't need to be known when this is `None`
    fallback: Option<(&'a dyn Fn(U) -> Option<&'a T>, U)>,
) -> Result<Vec<&'a T>, TargetInvalid<'a>>
where
    Iter: ExactSizeIterator<Item = &'a I>,
    I: AsRef<str> + 'a,
    T: TargetTrait<'a>,
{
    let targets_empty = targets.len() == 0;
    Ok(if !targets_empty {
        targets
            .map(|name| {
                T::for_name(name.as_ref()).ok_or_else(|| TargetInvalid {
                    name: name.as_ref().to_owned(),
                    possible: T::all().keys().cloned().collect(),
                })
            })
            .collect::<Result<_, _>>()?
    } else {
        let target = fallback
            .and_then(|(get_target, arg)| get_target(arg))
            .unwrap_or_else(|| {
                log::info!("falling back on default target ({})", T::DEFAULT_KEY);
                T::default_ref()
            });
        vec![target]
    })
}

pub fn call_for_targets_with_fallback<'a, Iter, I, T, U, F>(
    targets: Iter,
    fallback: &'a dyn Fn(U) -> Option<&'a T>,
    arg: U,
    f: F,
) -> Result<(), TargetInvalid<'a>>
where
    Iter: ExactSizeIterator<Item = &'a I>,
    I: AsRef<str> + 'a,
    T: TargetTrait<'a>,
    F: Fn(&T),
{
    let targets = get_targets(targets, Some((fallback, arg)))?;
    for target in targets {
        f(target);
    }
    Ok(())
}

pub fn call_for_targets<'a, Iter, I, T, F>(targets: Iter, f: F) -> Result<(), TargetInvalid<'a>>
where
    Iter: ExactSizeIterator<Item = &'a I>,
    I: AsRef<str> + 'a,
    T: TargetTrait<'a> + 'a,
    F: Fn(&T),
{
    let targets = get_targets::<_, _, _, ()>(targets, None)?;
    for target in targets {
        f(target);
    }
    Ok(())
}
