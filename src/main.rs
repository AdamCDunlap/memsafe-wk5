#[macro_use]
extern crate nom;

use nom::{digit,alphanumeric,space,IResult};
use std::str::from_utf8;
use std::io::{self, Write};
use std::rc::Rc;
use std::collections::HashMap;
use std::fmt;

type BranchName = String;
type CommitPayload = String;
#[derive(Debug)]
struct CommitReference {
    base: BranchName,
    offset: usize,
}

#[derive(Debug)]
enum Cmd {
    NewBranch(BranchName, CommitReference),
    NewCommit(CommitPayload, BranchName),
    DeleteBranch(BranchName),
    Examine,
}

named!(ospace<Option<&[u8]> >, opt!(space));

named!(newbranch<Cmd>,
    do_parse!(
        ospace >> tag!("new") >>
        space >> tag!("branch") >>
        space >> newname: alphanumeric >>
        space >> srcname: alphanumeric >>
        offset: opt!(complete!(do_parse!(
            ospace >> char!('~') >>
            ospace >> offset: digit >>
            (from_utf8(offset).unwrap().parse().unwrap())
        ))) >>
        (Cmd::NewBranch(from_utf8(newname).unwrap().to_string(),
         CommitReference{base: from_utf8(srcname).unwrap().to_string(),
                         offset: offset.unwrap_or(0)}))
    )
);

named!(deletebranch<Cmd>,
    do_parse!(
        ospace >> tag!("delete") >>
        space >> tag!("branch") >>
        space >> name: alphanumeric >>
        (Cmd::DeleteBranch(from_utf8(name).unwrap().to_string()))
    )
);

named!(newcommit<Cmd>,
    do_parse!(
        ospace >> tag!("new") >>
        space >> tag!("commit") >>
        space >> payload: do_parse!(
                    char!('\'') >>
                    data: take_until_and_consume!("'") >>
                    (data)
                ) >>
        space >> branchname: alphanumeric >>
        (Cmd::NewCommit(from_utf8(payload).unwrap().to_string(),
                        from_utf8(branchname).unwrap().to_string()))
    )
);

named!(examine<Cmd>,
   do_parse!(
       ospace >> tag!("examine") >>
       (Cmd::Examine)
    )
);

named!(command<Cmd>, alt_complete!(
    newbranch |
    deletebranch |
    newcommit |
    examine
));

enum DatabaseError {
    BranchDoesntExist(BranchName),
    CommitNotDeepEnough(BranchName, usize),
}
impl fmt::Display for DatabaseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &DatabaseError::BranchDoesntExist(ref name) =>
                write!(f, "Branch ``{}'' doesn't exist", name),
            &DatabaseError::CommitNotDeepEnough(ref name, depth) =>
                write!(f, "Branch ``{}'' does not go back {} commit{}",
                       name, depth, if depth == 1 {""} else {"s"}),
        }
    }
}

#[derive(Debug)]
struct Commit {
    parent: Option<Rc<Box<Commit>>>,
    data: String,
}
impl Drop for Commit {
    fn drop(&mut self) {
        println!("'{}' deleted", self.data);
    }
}

fn execute_cmd(database: &mut HashMap<String, Rc<Box<Commit>>>, c: &Cmd)
        -> Result<(), DatabaseError>{
    match c {
        &Cmd::NewBranch(ref name, CommitReference{ref base, offset}) => {
            let mut ci = try!(database.get(base)
                              .ok_or(DatabaseError::BranchDoesntExist(
                                      base.clone()))
                             ).clone();
            for _ in 0..offset {
                ci = try!(ci.parent.clone().
                          ok_or(DatabaseError::CommitNotDeepEnough(base.clone(),
                                                                   offset)));
            }
            println!("{} -> '{}'", name, ci.data);
            database.insert(name.clone(), ci);
            Ok(())
        },
        &Cmd::NewCommit(ref data, ref branch) => {
            let parent = try!(database.get(branch)
                              .ok_or(DatabaseError::BranchDoesntExist(
                                      branch.clone()))
                              ).clone();
            
            println!("{} -> '{}'", branch, data);
            let ci = Commit{parent: Some(parent.clone()), data: data.clone()};
            database.insert(branch.clone(), Rc::new(Box::new(ci)));
            Ok(())
        },
        &Cmd::DeleteBranch(ref branch) => {
            database.remove(branch).ok_or(
                DatabaseError::BranchDoesntExist(branch.clone())).map(
                    |_| println!("{} deleted", branch))
        },
        &Cmd::Examine => {
            println!("{:#?}", database);
            Ok(())
        }
    }
}

const ROOT_BRANCH_NAME: &'static str = "master";

fn main() {

    let mut database = HashMap::new();
    let basecommit = Commit{parent:None, data: std::env::args().nth(1).expect(
            "Give command line argument for master commit data")};
    println!("{} -> '{}'", ROOT_BRANCH_NAME, basecommit.data);
    database.insert(ROOT_BRANCH_NAME.to_string(), Rc::new(Box::new(basecommit)));

    loop {
        let mut line = String::new();
        println!("");
        print!("> ");
        io::stdout().flush().unwrap();
        if io::stdin().read_line(&mut line).map(|l| l == 0).unwrap_or(false) {
            break;
        }
        match command(line.as_str().trim().as_bytes()) {
            IResult::Done(remaining, ref res) if remaining.len() == 0 =>
                match execute_cmd(&mut database, res) {
                    //Ok(()) => println!("{:#?}", database),
                    //Err(e) => println!("Error: {}", e),

                    Ok(()) => (),
                    Err(_) => println!("Error"),
                },
            //e => println!("Error: {:?}", e),
            _ => println!("Error"),
        }
    }
}
