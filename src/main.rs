use std::fs::File;
use std::env;
use std::assert;
use std::process::exit;
use std::num::Wrapping;
use std::io::{Read, Write, stdin, stdout/*, stderr*/};
use std::collections::HashMap;


const OPCODE_SIZE:usize = 32;

const TOKENS:[u8;16] = [ //encoding 1 nibble pertoken, 2 per byte
    b'!', // HIGH ALL write new value to stack with all bits set to 1 (-1)
    b'^', // XOR
    b'|', // OR
    b'&', // AND

    b'+', // ADD
    b'-', // SUB
    b'*', // MUL (returns 2 stack numbners)
    b'/', // DIV (/0  = 0, can be used for test by performing x/x (0 when equal 1 when not equal))

    b'$', // Switch stack
    b'~', // Head stackA <-> head stackB 
    b'=', // Duplicate top value
    b'@', // SKIP aka JUMP (how much to jump extra after CP increases)

    b'?', // READ fs to stack
    b'.', // WRITE write byte to fs
    b'0', // SHL 1
    b'1', // SHL 1; | 1
];

fn main() -> Result<(),std::io::Error> {
// fn main<E:From<std::io::Error> + std::fmt::Debug>() -> Result<(),E> {

    let mut args = env::args();
    args.next();

    let mut debug = false;
    let mut strict = false;

    let params:Vec<String> = args.collect();
    let mut filename = "".to_owned();

    let mut mode = Mode::RUN;
    enum Mode{
        RUN,
        COMPILE,
        BYTECODE,
        DUMP,
        ELF64,
        // PE64,
    }


    for p in params{
        let param = p.as_str();

        if param.starts_with("--"){
            match param{
                "--help" => {
                    eprintln!("Usage stackofstacks [--debug, --compile, --bytecode] FILENAME");
                    eprintln!("  --debug     Shows debug / trace on STDERR while runniogn program");
                    eprintln!("  --dump      Dumps the raw (macro expanded) code");
                    eprintln!("  --strict    Aborts when popping from empty stack or accessing uninitialised ram");
                    eprintln!("  --compile   Compiles program to bytecode (emitted on STDOUT)");
                    eprintln!("  --bytecode  Runs compiled bytecode instead of text");
                    eprintln!("  --elf64     Compiles to an native linux 64 bit elf executable");
                },                
                "--debug" => {
                    debug = true;
                },
                "--dump" => {
                    mode = Mode::DUMP;
                },                
                "--strict" => {
                    strict = true;
                },
                "--compile" => {
                    mode = Mode::COMPILE;
                },
                "--bytecode" => {
                    mode = Mode::BYTECODE;
                },
                "--elf64" => {
                    mode = Mode::ELF64;
                },
                // Lets diable this untill it works
                // "--pe64" => {
                //     mode = Mode::PE64;
                // },                
                _ => {
                    eprintln!("Unknown option '{}' !", param);
                    exit(1);
                }
            }   
        }else{
            if filename != ""{
                eprintln!("Only one filename allowed!");
                exit(1);                    
            }
            filename = p;         
        }


    }

    if filename == ""{
        eprintln!("No filename specified!");
        exit(1);
    }

    let mut file = match File::open(filename)
    {
        Ok(f) => f,
        Err(_) => {
            eprintln!("Error opening file");
            exit(1);
        }
    };

    let size:usize = match file.metadata()
    {
        Ok(l) => l,
        Err(_) => {
            eprintln!("File has no filesize");
            exit(1);
        }
    }.len().try_into().unwrap();
    
    let mut script_bytes:Vec<u8> = Vec::with_capacity(size.try_into().unwrap());
    script_bytes.resize(size, 0);
    file.read(&mut script_bytes)?;

    match mode{
        Mode::RUN => { 
            // println!("{:?}", &tokenise(script_bytes));
            // println!("{}", std::str::from_utf8(&parse(&tokenise(script_bytes))).unwrap());
            run(&parse(&tokenise(script_bytes)), debug, strict);
        },
        Mode::COMPILE => { 
            let mut out = stdout().lock();
            out.write( &compile(&parse(&tokenise(script_bytes))) )?; 
        },
        Mode::BYTECODE => { 
            run(&bytecode(&parse(&tokenise(script_bytes))), debug, strict); 
        },
        Mode::DUMP => { 
            let pure_script = parse(&tokenise(script_bytes));

             //input is already screened to not contain non-ascii characters by the tokenise funtion
            let string = std::str::from_utf8(&pure_script).expect("INTERPRETER's FAULT: The function 'tokenise' should have checked for non ascii characters!");
            let mut index = 0;

            for token in string.chars(){
                eprintln!("0x{:#018X}:  {}", index, token);
                index += 1;
            }
        },
        Mode::ELF64 => { 
            emit_elf64(&compile(&parse(&tokenise(script_bytes))))?; 
        },
        // Mode::PE64 => { 
        //     emit_pe64(&compile(&parse(&tokenise(script_bytes))))?; 
        // },
    }

    Ok(())

}

fn expand(input:&Vec<u8>, labels:&HashMap<Vec<u8>,usize>, _index:&usize)->Vec<u8>{

    //input is already screened to not contain non-ascii characters by the tokenise funtion
    // let string = std::str::from_utf8(input).expect("INTERPRETER's FAULT: The function 'tokenise' should have checked for non ascii characters!");

    #[derive(Debug)]
    enum Token{
        Number(Vec<u8>),
        Operator(u8),
    }

    let mut tokens:Vec<Token> = vec!();
    let mut buffer:Vec<u8> = vec!();

    for c in input{

        if [b'+',b'-'].contains(c){
            if buffer.len() > 0 {
                tokens.push(Token::Number(buffer));
                buffer = vec!();
            }

            tokens.push(Token::Operator(*c));            
        }else{
            buffer.push(*c);
        }

    }
    if buffer.len() > 0 {
        tokens.push(Token::Number(buffer));
        // buffer = vec!();
    }

    //check well formed ness
    //check for empty macro
    if tokens.len() < 1{
        eprintln!("Macro parsing error: Empty macro!");
        exit(1);        
    }

    //if start with operator eg: -1 the  prepend 0 so -4 becomes 0-4
    if matches!(tokens[0], Token::Operator(_)){
        tokens = {let mut x = vec!(Token::Number(vec!(b'0'))); x.extend(tokens); x};
    }

    //check if macro ends with Number
    if !matches!(tokens[tokens.len()-1], Token::Number(_)){
        eprintln!("Macro parsing error: Macro cannot end with operator: [{}]", std::str::from_utf8(input).unwrap());
        exit(1);    
    }

    //chekc if macro has format of: number (operator number)*
    let mut should_be_number = true;
    for token in &tokens{

        match token{
            Token::Number(_) => {
                if !should_be_number{
                    eprintln!("Macro parsing error: Unexpected (extra) Number in macro: [{}]", std::str::from_utf8(input).unwrap());
                    exit(1);        
                }
            },
            Token::Operator(_) => {
                if should_be_number{
                    eprintln!("Macro parsing error: Unexpected (extra) Operator in macro: [{}]", std::str::from_utf8(input).unwrap());
                    exit(1);        
                }
            },
        }

        should_be_number = !should_be_number;
    }

    #[derive(Debug)]
    enum T2Token{
        Int(i64),
        Add,
        Sub,
    }

    //Validate all tokens individually, and return a list of T2 tokens
    fn resolve_tokens(tokens:Vec<Token>, labels:&HashMap<Vec<u8>,usize>) -> Vec<T2Token>{
        let mut out = vec!();

        for token in &tokens{

            let new_token = match token{
                Token::Number(v) => {

                    let s = std::str::from_utf8(v).unwrap();

                    let int = match s.parse(){
                        Ok(int) => int, //int parsed
                        Err(_) => {

                            let int;
                            // You want to use from_str_radix. It's implemented on the integer types.
                            if v.len()==3 && v[0] == b'\'' && v[2] == b'\'' { //matches 'X' notation
                                int = i64::from(v[1]);

                            }else if v[0] == b'0' && (v[1] == b'b' || v[1] == b'x' || v[1] == b'o' || v[1] == b'd'){
                                let number_string = &s[2..];
                                let radix = match v[1]{
                                    b'b' => 2,
                                    b'o' => 8,
                                    b'd' => 10, //Just to be complete
                                    b'x' => 16,
                                    _ => {panic!();},
                                };

                                int = match i64::from_str_radix(number_string, radix){
                                    Ok(i) => i,
                                    Err(_) => {
                                        eprintln!("Macro parsing error: '{}' is an invalid number representation", s);
                                        exit(1);    
                                    },
                                }
                            }else{
                                int = match labels.get(v){
                                    Some(u) => *u as i64,
                                    None => {
                                        eprintln!("Macro parsing error: Label '{}' not found in labels", s);
                                        exit(1);                                  
                                    }
                                };                                
                            }

                            int
                        },
                    };


                    T2Token::Int(int)
                }
                Token::Operator(c) => {
                    match c{
                        b'+' => T2Token::Add,
                        b'-' => T2Token::Sub,
                        _ => {panic!("INTERPRETER's FAULT: Operator token should only cointain +/- !");}
                    }
                }
            };

            out.push(new_token);
        }

        out

    }

    let t2tokens = resolve_tokens(tokens,labels);

    // println!("{:?}", t2tokens);

    let mut it = t2tokens.iter();

    let T2Token::Int(mut acc) = it.next().unwrap() else {panic!()};

    loop{
        let Some(op_token) = it.next() else {break};
        let T2Token::Int(num) = it.next().unwrap() else {panic!()};

        match op_token{
            T2Token::Add => {
                acc += num;
            },
            T2Token::Sub => {
                acc -= num;
            },
            T2Token::Int(_) => {panic!();}
        }
    }


    // Here we make a numbe rinot a list of opcodes
    // we CANNOT and WILL not make this variable length..
    // since then everything shifts, compile rno longer streaming, manual offsets break...
    let ret = format!("!{:064b}", acc);
    assert!(ret.len() == 65); //Must be 65 characters wide

//     let mut ret:Vec<u8> = vec!();
//     const HIGH_BIT: i64 = i64::MIN; //This is only highest bit = true
//     let num:bool = (acc & HIGH_BIT) != 0;

//     let mut leading:usize = 1;
//     for i in 0..63{
//         acc <<= 1;
//         if ((acc & HIGH_BIT)!=0) != num {break}
//         leading += 1;
//     }

//         match num{
//             false => {  //positive number leading 0
//                 ret.push(b'!');
//                 ret.push(b'!');
//                 ret.push(b'^');
//             },
//             true => { //negtaive number leading 1
//                 ret.push(b'!');
//             },
//         };

//     //TODO:! implement bit conversion for 2-complement negative

//     for i in 0..64-leading{

//         let chr = match (acc & HIGH_BIT)!=0{
//             true => b'1',
//             false => b'0',
//         };
//         ret.push(chr);

//         acc <<= 1;
//     }


// for token in ret.iter(){
    for token in ret.bytes(){
        if !TOKENS.contains(&token){
            panic!("INTERPRETER's FAULT: Invalid tokens in macro output!");
        }
    }
    return Vec::from(ret);



}

#[derive(Debug)]
enum Token{
    Script(Vec<u8>),
    Macro(Vec<u8>),
    Label(Vec<u8>),
}


fn tokenise(script_bytes:Vec<u8>) -> Vec<Token>{

    #[derive(Copy, Clone)]
    enum State{
        Script,
        Comment,
        Macro,
        Label,
    }

    // let mut pure_script:Vec<u8> = vec!();

    let mut tokenised_script:Vec<Token>  = vec!();
    let mut buffer:Vec<u8> = vec!();

    let mut state = State::Script;
    let mut line_count = 1;
    let mut char_count = 1;

    for token in script_bytes{

        if token >= 0x80 {
            eprintln!("Parsing error: Illegal (Non ASCII) character found at {}:{}", line_count, char_count);
            exit(1);
        }
        if token != 0x0D{ //CR not counted
            char_count += 1;
        }
        if token == 0x0A{
            char_count = 1;
            line_count += 1;
        }

        
        match state{
            State::Script =>{
                // println!("{:}: {:?}", "Source", std::str::from_utf8(&buffer).unwrap());

                if TOKENS.contains(&token){
                    buffer.push(token);
                }else{
                    match token{
                        b'#' => {
                            tokenised_script.push(Token::Script(buffer));
                            buffer = vec!();
                                                        
                            state = State::Comment;
                        },
                        b'[' => {
                            tokenised_script.push(Token::Script(buffer));
                            buffer = vec!();

                            state = State::Macro;
                        },
                        b':' => {
                            tokenised_script.push(Token::Script(buffer));
                            buffer = vec!();

                            state = State::Label;
                        },                        
                        _ => (), //preceived as comment
                    }
                }
            },
            State::Comment =>{
                // println!("{:}: {:?}", "Comment", std::str::from_utf8(&buffer).unwrap());

                match token{
                    b'\n' => {
                        state = State::Script;
                    },
                    _ => (), //preceived as comment
                }                
            },
            State::Macro =>{
                // println!("{:}: {:?}", "Macro", std::str::from_utf8(&buffer).unwrap());
                match token{
                    b']' => {

                        tokenised_script.push(Token::Macro(buffer));
                        buffer = vec!();

                        state = State::Script;
                    },
                    b => {
                        buffer.push(b);
                    }
                }                
            }
            State::Label =>{ //97=122
                // println!("{:}: {:?}", "Label", std::str::from_utf8(&buffer).unwrap());
                if (97 <= token) && (token <= 122){
                    buffer.push(token);
                }else{

                    tokenised_script.push(Token::Label(buffer));
                    buffer = vec!();

                    state = State::Script;
                }   
            }
        }


    }

    match state{
        State::Script =>{
            tokenised_script.push(Token::Script(buffer));
        },
        State::Comment =>{
            //not needed here, its just discarded while interwoven with Source state
        },
        State::Macro =>{
            eprintln!("Macro violation: Macro is not closed by EOF.");
            exit(1);
        },
        State::Label =>{
            tokenised_script.push(Token::Label(buffer));
        },
    }

    tokenised_script
}

//parse to pure_script
fn parse(tokens:&Vec<Token>) -> Vec<u8>{

    let mut labels:HashMap<Vec<u8>, usize> = HashMap::new();
    let mut index = 0;
    let mut pure_script:Vec<u8> = vec!();

    for token in tokens{
        match token{
            Token::Script(v) => {
                index += v.len();
            },
            Token::Macro(_) => {
                //And heres the crux, we need to know NOW how long expanded macro size will be, and macro needs the label offset chicken and egg story
                //For now macro's have output size fixed at 65 !+binary number
                index += 65;
            },
            Token::Label(v) => {
                labels.insert(v.to_vec(), index);
            },
        }
    }

    for token in tokens{
        match token{
            Token::Script(v) => {
                // index += len(v);
                pure_script.extend(v);
            },
            Token::Macro(v) => {
                let expanded_v = expand(&v, &labels, &index);
                index += expanded_v.len();
                pure_script.extend(expanded_v);
            },
            Token::Label(_) => {},
        }
    }

    // println!("{}", std::str::from_utf8(&pure_script).unwrap());
    return pure_script;
}

fn compile(code:&Vec<u8>) -> Vec<u8>{
    let mut opcodes:HashMap<u8, u8> = HashMap::new();

    let mut high_bits = true;
    let mut out:Vec<u8> = vec!();

    for (index, token) in TOKENS.iter().enumerate(){
        opcodes.insert(*token, index as u8);
    }

    //Since opcode 0 (PUSH(-1)) always works and is benign on itself, we dont care is lasat nibble contains just that.
    for token in code{
        if high_bits{
            out.push( *opcodes.get(token).unwrap() << 4 );
        }else{
            let i:usize = out.len()-1;
            out[i] |= *opcodes.get(token).unwrap();
        }
        high_bits = !high_bits;
    }

    out
}


fn bytecode(bytecode:&Vec<u8>) -> Vec<u8>{
    let mut code:Vec<u8> = vec!();

    for byte in bytecode{
        code.push(TOKENS[usize::from(byte>>4)]);
        code.push(TOKENS[usize::from(byte&0b1111)]);
    }

    code
}


trait Oos{
    fn oos(self:&Self, infinite_stack:&bool)->i64;
}

impl Oos for Option<i64>{
    fn oos(self:&Self, strict:&bool) -> i64{
        match self{

            Some(v) => *v,
            None => {
                if *strict{
                    eprintln!("Strict mode violation: Stack depleted");
                    exit(1);
                }else{
                    -1
                }
            }

        }

    }    
}

fn run(code:&Vec<u8>, debug:bool, strict:bool){

    if code.len() == 0{return}

    let mut index = 0;
    // let mut ram:HashMap<i64, i64> = HashMap::new();

    let mut stacks: [Vec<i64>;2] = [vec!(),vec!()];
    let mut stack: &mut Vec<i64>;
    let mut stack_index = 0;
    stack = &mut stacks[stack_index];

    loop{

        if debug{
            eprintln!("{:?}", &stack);
            stack = &mut stacks[!stack_index&1];
            eprintln!("{:?}", &stack);
            stack = &mut stacks[stack_index&1];
            eprintln!("{:#018X}: {}", index, code[index] as char);
        }

        match code[index]{
            b'$' => {//Switch stack
                stack_index = !stack_index&1;
                stack = &mut stacks[stack_index];
            }
            b'~' => {//Xchange stack heads
                let a = stack.pop().oos(&strict);
                stack = &mut stacks[!stack_index&1];
                let b = stack.pop().oos(&strict);
                
                stack.push(a);
                stack = &mut stacks[stack_index];
                stack.push(b);
            }
            b'=' => {// Duplicate top value
                let a = stack.pop().oos(&strict);
                stack.push(a);
                stack.push(a);
            }            
            b'+' => {
                let b = stack.pop().oos(&strict);
                let a = stack.pop().oos(&strict);
                stack.push( (Wrapping(a)+Wrapping(b)).0 );
            }
            b'-' => {
                let b = stack.pop().oos(&strict);
                let a = stack.pop().oos(&strict);
                stack.push( (Wrapping(a)-Wrapping(b)).0 );
            }
            b'*' => { // MULTIPLY stack:[a,b] -> [low, high]
                let b = stack.pop().oos(&strict);
                let a = stack.pop().oos(&strict);
                let x = i128::from(a)*i128::from(b);
                stack.push(x as i64);
                // stack.push((x >> 64) as i64); //just lose the eccess
            }
            b'/' => {
                let b = stack.pop().oos(&strict);
                let a = stack.pop().oos(&strict);
                if b != 0{
                    stack.push(a/b);
                }else{
                    stack.push(0); //div by 0 is 0 by design (can replace test)
                }
            }
            b'|' => {
                let b = stack.pop().oos(&strict);
                let a = stack.pop().oos(&strict);
                stack.push( a | b );
            }
            b'&' => {
                let b = stack.pop().oos(&strict);
                let a = stack.pop().oos(&strict);
                stack.push( a & b );
            }
            b'^' => {
                let b = stack.pop().oos(&strict);
                let a = stack.pop().oos(&strict);
                stack.push( a ^ b );
            }
            b'!' => {
                stack.push( -1 );
            }
            b'1' => {
                let a = stack.pop().oos(&strict);
                stack.push((a << 1) | 0b1);
            }
            b'0' => {
                let a = stack.pop().oos(&strict);
                stack.push(a << 1);
            }
            b'@' => {
                let a = stack.pop().oos(&strict);
                if a == -1 {break} // This jumps back to the same opcode JMP again, so never needed, reserved for exit
                index = (Wrapping(index)+Wrapping(a as usize)).0;
            }
            b'?' => {
                let mut buffer:[u8;1] = [0];
                let stack_value = match stdin().lock().read_exact(&mut buffer){
                    Ok(_) => buffer[0] as i64,
                    Err(_) => -1,   
                };
                stack.push(stack_value);
            }
            b'.' => {
                let ch = stack.pop().oos(&strict);

                let buffer:[u8;1] = [ch as u8];
                let _ = stdout().lock().write(&buffer);
            }            
            _ => {
                panic!("INTERPRETER's FAULT: Invalid token!")
            }
        }

        index = (Wrapping(index)+Wrapping(1usize)).0;
        if index >= code.len() {
            if strict{
                eprintln!("Strict mode violation: Outside of code memmory (offset: 0x{:#018X})", index);
                exit(1);
            }else{

            }   index = 0;
        }
    }

    if debug{
        eprintln!("{:?}", &stack);
        stack = &mut stacks[!stack_index&1];
        eprintln!("{:?}", &stack);
    }
    

}

fn emit_elf64(bytecode: &Vec<u8>) -> Result<(),std::io::Error>{

    let opcodes_bytes = include_bytes!("../templates/opcodes.template");
    let prologue_bytes = include_bytes!("../templates/prologue_elf64.template");
    let epilogue_bytes = include_bytes!("../templates/epilogue_elf64.template");

    let mut asm_index:Vec<u8> = vec!();

    //convert to indices in template
    for byte in bytecode{
        asm_index.push((byte >> 4) & 0xF);
        asm_index.push(byte & 0xF);
    }

    std::io::stdout().write_all(prologue_bytes)?;
    
    for opcode in &asm_index{
        let start = OPCODE_SIZE*(*opcode) as usize;
        let end = OPCODE_SIZE*(*opcode+1) as usize;
        std::io::stdout().write_all(&opcodes_bytes[start..end])?;
    }

    std::io::stdout().write_all(epilogue_bytes)?;
    Ok(())
}

// fn emit_elf64(bytecode: &Vec<u8>) -> Result<(),std::io::Error>{

//     fn get_embedded_templates() -> (&'static [u8], &'static [u8], &'static [u8]) {
//         const OPCODE_SIZE: usize = 32;
        
//         // De bestanden worden tijdens 'cargo build' ingeladen in de binary
//         let opcodes_bytes = include_bytes!("../templates/opcodes.template");
//         let prologue_bytes = include_bytes!("../templates/prologue_elf64.template");
//         let epilogue_bytes = include_bytes!("../templates/epilogue_elf64.template");

//         // Validatie (wordt geëvalueerd tijdens runtime, maar is nu een simpele assert)
//         assert!(
//             opcodes_bytes.len() == OPCODE_SIZE * 16,
//             "Resource error: opcodes.template file corrupt. (size: {} != {})",
//             opcodes_bytes.len(),
//             OPCODE_SIZE * 16
//         );

//         (prologue_bytes, opcodes_bytes, epilogue_bytes)
//     }


//     let mut asm_index:Vec<u8> = vec!();

//     //convert to indices in template
//     for byte in bytecode{
//         asm_index.push((byte >> 4) & 0xF);
//         asm_index.push(byte & 0xF);
//     }

//     let (prologue_bytes, opcodes_bytes, epilogue_bytes) = get_embedded_templates();

//     let mut padding:Vec<u8> = vec!();

//     while ( (prologue_bytes.len() + padding.len() )  % OPCODE_SIZE) != 0{
//         padding.push(0x90);
//     }


//     std::io::stdout().write_all(prologue_bytes)?;
//     std::io::stdout().write_all(&padding)?;
    
//     for opcode in &asm_index{
//         let start = OPCODE_SIZE*(*opcode) as usize;
//         let end = OPCODE_SIZE*(*opcode+1) as usize;
//         std::io::stdout().write_all(&opcodes_bytes[start..end])?;
//     }

//     std::io::stdout().write_all(epilogue_bytes)?;
//     Ok(())
// }



//lets disabl ethios so we dont include junk data not used yet

// fn emit_pe64(bytecode: &Vec<u8>) -> Result<(), std::io::Error> {
//     const OPCODE_SIZE: usize = 32;
//     const FILE_ALIGNMENT: usize = 512;

//     fn get_embedded_templates() -> (&'static [u8], &'static [u8], &'static [u8]) {
//         let opcodes_bytes = include_bytes!("../templates/opcodes.template");
//         let prologue_bytes = include_bytes!("../templates/prologue_pe64.template");
//         let epilogue_bytes = include_bytes!("../templates/epilogue_pe64.template");

//         assert!(
//             opcodes_bytes.len() == OPCODE_SIZE * 16,
//             "Resource error: opcodes.template file corrupt."
//         );

//         (prologue_bytes, opcodes_bytes, epilogue_bytes)
//     }

//     let mut asm_index: Vec<u8> = vec![];
//     for byte in bytecode {
//         asm_index.push((byte >> 4) & 0xF);
//         asm_index.push(byte & 0xF);
//     }

//     let (prologue_bytes, opcodes_bytes, epilogue_bytes) = get_embedded_templates();

//     // 1. Bouw de complete binary eerst op in het geheugen (een buffer)
//     let mut binary_buffer: Vec<u8> = prologue_bytes.to_vec();

//     // Padding tussen proloog en eerste opcode voor de 32-byte uitlijning
//     while (binary_buffer.len() % OPCODE_SIZE) != 0 {
//         binary_buffer.push(0x90);
//     }

//     // Voeg alle opcodes toe
//     for opcode in &asm_index {
//         let start = OPCODE_SIZE * (*opcode) as usize;
//         let end = OPCODE_SIZE * (*opcode + 1) as usize;
//         binary_buffer.extend_from_slice(&opcodes_bytes[start..end]);
//     }

//     // Voeg de epiloog toe
//     binary_buffer.extend_from_slice(epilogue_bytes);

//     // 2. Windows Specifieke Padding: Het hele bestand MOET een veelvoud van 512 bytes zijn
//     while (binary_buffer.len() % FILE_ALIGNMENT) != 0 {
//         binary_buffer.push(0x00); // Nullen aan het einde van het bestand is prima
//     }

//     let total_file_size = binary_buffer.len() as u32;

//     // 3. Patch de PE-Header met de exacte nieuwe groottes!
//     // We overschrijven de hardcoded waarden in de template met de echte runtime waarden.
    
//     // Size of Code (Offset 0x74 in een standaard PE32+ header vanaf het begin van het bestand)
//     let size_of_code = total_file_size - 512;
//     binary_buffer[0x74..0x78].copy_from_slice(&size_of_code.to_le_bytes());

//     // Size of Image (Offset 0x90: Hoeveel virtueel geheugen er nodig is, we ronden af op 4096 alignment)
//     let size_of_image = ((total_file_size + 4095) / 4096) * 4096;
//     binary_buffer[0x90..0x94].copy_from_slice(&size_of_image.to_le_bytes());

//     // Size of Raw Data in de .text Section Header (Offset 0x110)
//     let size_of_raw_data = total_file_size - 512;
//     binary_buffer[0x110..0x114].copy_from_slice(&size_of_raw_data.to_le_bytes());

//     // Virtual Size in de .text Section Header (Offset 0x10C)
//     binary_buffer[0x10C..0x110].copy_from_slice(&size_of_raw_data.to_le_bytes());


//     // 4. Schrijf de volledig gepatchte en uitgelijnde binary naar stdout
//     std::io::stdout().write_all(&binary_buffer)?;
//     Ok(())
// }
