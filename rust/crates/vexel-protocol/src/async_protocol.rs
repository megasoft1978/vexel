use vexel_json::{JsonBuffer, JsonDocument, JsonType, MAX_BYTES};

const TASK_ID_LIMIT: usize = 1024;
const TASK_TTL_LIMIT: u64 = 900_000;
const PROGRESS_TOKEN_LIMIT: usize = 1024;
const PROGRESS_MESSAGE_LIMIT: usize = 127;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AsyncDisposition { Unhandled, Result, Error, Deferred, Failed }
impl Default for AsyncDisposition { fn default() -> Self { Self::Unhandled } }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AsyncPayloadKind { ToolResult, Error, TaskGet, TaskCancel }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgressError { InvalidToken, InvalidProgress, InvalidMessage, Buffer }

pub fn tasks_ready<B: crate::AsyncBackend + ?Sized>(backend: Option<&B>) -> bool {
    backend.is_some_and(crate::AsyncBackend::tasks_available)
}

pub fn validate_async_payload(data: &[u8], kind: AsyncPayloadKind) -> Result<bool, ()> {
    if data.is_empty() || data.len() > MAX_BYTES { return Err(()); }
    let doc = JsonDocument::parse(data).map_err(|_| ())?;
    let root = doc.root();
    if doc.token(root).is_none_or(|t| t.kind != JsonType::Object) { return Err(()); }
    let task = match kind {
        AsyncPayloadKind::Error => valid_error(&doc, root),
        AsyncPayloadKind::ToolResult => {
            if string_is(&doc, root, b"resultType", b"task") { valid_create_task(&doc, root) }
            else { valid_call_result(&doc, root) }
        }
        AsyncPayloadKind::TaskGet => string_is(&doc, root, b"resultType", b"complete") && valid_get_task(&doc, root),
        AsyncPayloadKind::TaskCancel => valid_cancel_ack(&doc, root),
    };
    if task { Ok(kind == AsyncPayloadKind::ToolResult && string_is(&doc, root, b"resultType", b"task")) } else { Err(()) }
}

fn get(doc: &JsonDocument<'_>, object: usize, key: &[u8]) -> Option<usize> { doc.object_get(object, key) }
fn string_is(doc: &JsonDocument<'_>, object: usize, key: &[u8], expected: &[u8]) -> bool { get(doc, object, key).is_some_and(|v| doc.string_equals(v, expected)) }
fn optional_object(doc: &JsonDocument<'_>, object: usize, key: &[u8]) -> bool { get(doc, object, key).is_none_or(|v| doc.token(v).is_some_and(|t| t.kind == JsonType::Object)) }
fn valid_string(doc: &JsonDocument<'_>, token: Option<usize>, nonempty: bool) -> bool {
    let Some(token) = token else { return false; };
    let Some(bytes) = doc.decoded_string(token) else { return false; };
    (!nonempty || !bytes.is_empty()) && !bytes.contains(&0)
}
fn valid_timestamp(doc: &JsonDocument<'_>, object: usize, key: &[u8]) -> bool {
    let Some(token) = get(doc, object, key) else { return false; };
    let Some(v) = doc.decoded_string(token) else { return false; };
    let Ok(s) = std::str::from_utf8(&v) else { return false; };
    if v.contains(&0) || s.len() < 20 || s.len() > 63 || !s.is_ascii() { return false; }
    let b = s.as_bytes();
    if b[4] != b'-' || b[7] != b'-' || b[10] != b'T' || b[13] != b':' || b[16] != b':' { return false; }
    for (i, c) in b.iter().take(19).enumerate() { if !matches!(i, 4|7|10|13|16) && !c.is_ascii_digit() { return false; } }
    let n = |a: usize, z: usize| -> u32 { s[a..z].parse().unwrap_or(u32::MAX) };
    let (year, month, day, hour, minute, second) = (n(0,4),n(5,7),n(8,10),n(11,13),n(14,16),n(17,19));
    if year == 0 || !(1..=12).contains(&month) || hour > 23 || minute > 59 || second > 60 { return false; }
    let mdays = [31, if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) {29} else {28},31,30,31,30,31,31,30,31,30,31];
    if day == 0 || day > mdays[(month-1) as usize] { return false; }
    let mut i = 19;
    if b.get(i) == Some(&b'.') { i += 1; let start=i; while b.get(i).is_some_and(u8::is_ascii_digit) { i+=1; } if i==start { return false; } }
    if b.get(i) == Some(&b'Z') { return i+1 == b.len(); }
    if i.checked_add(6) != Some(b.len()) || !matches!(b.get(i), Some(b'+')|Some(b'-')) || b.get(i+3) != Some(&b':') { return false; }
    for (j,c) in b.iter().enumerate().skip(i+1) { if j != i+3 && !c.is_ascii_digit() { return false; } }
    let zone_hour = u32::from(b[i+1]-b'0')*10+u32::from(b[i+2]-b'0');
    let zone_minute = u32::from(b[i+4]-b'0')*10+u32::from(b[i+5]-b'0');
    zone_hour <= 14 && zone_minute <= 59 && (zone_hour != 14 || zone_minute == 0)
}
fn valid_error(doc: &JsonDocument<'_>, object: usize) -> bool {
    let Some(code) = get(doc, object, b"code") else { return false; };
    doc.token(code).is_some_and(|t| t.kind == JsonType::Number) && doc.raw_token(code).is_some_and(|raw| !raw.iter().any(|b| matches!(b,b'.'|b'e'|b'E'))) && valid_string(doc, get(doc, object,b"message"), true)
}
fn valid_content(doc: &JsonDocument<'_>, token: Option<usize>) -> bool {
    let Some(token)=token else{return false};
    let Some(array)=doc.token(token) else{return false};
    if array.kind != JsonType::Array { return false; }
    let mut block=array.first_child;
    while let Some(i)=block {
        let Some(b)=doc.token(i) else{return false}; if b.kind!=JsonType::Object || !optional_object(doc,i,b"annotations"){return false;}
        let Some(ty)=get(doc,i,b"type") else{return false}; if !valid_string(doc,Some(ty),true){return false;}
        if doc.string_equals(ty,b"text") { if !valid_string(doc,get(doc,i,b"text"),false){return false;} }
        else if doc.string_equals(ty,b"image")||doc.string_equals(ty,b"audio") { if !valid_string(doc,get(doc,i,b"data"),true)||!valid_string(doc,get(doc,i,b"mimeType"),true){return false;} }
        else if doc.string_equals(ty,b"resource") {
            let Some(r)=get(doc,i,b"resource") else{return false};
            if doc.token(r).is_none_or(|t| t.kind!=JsonType::Object)||!optional_string(doc,r,b"uri",true)||!optional_string(doc,r,b"mimeType",true){return false;}
            if !valid_string(doc,get(doc,r,b"uri"),true) {return false;}
            let text=get(doc,r,b"text"); let blob=get(doc,r,b"blob");
            if text.is_some()==blob.is_some() || text.is_some_and(|v|!valid_string(doc,Some(v),false)) || blob.is_some_and(|v|!valid_string(doc,Some(v),false)){return false;}
        }
        else if doc.string_equals(ty,b"resource_link") {
            if !valid_string(doc,get(doc,i,b"uri"),true)||!valid_string(doc,get(doc,i,b"name"),true)||!optional_string(doc,i,b"title",false)||!optional_string(doc,i,b"description",false)||!optional_string(doc,i,b"mimeType",true){return false;}
            if get(doc,i,b"icons").is_some_and(|v|doc.token(v).is_none_or(|t|t.kind!=JsonType::Array)) || get(doc,i,b"size").is_some_and(|v|doc.uint(v).is_none()){return false;}
        } else {return false;}
        block=b.next_sibling;
    }
    true
}
fn optional_string(doc:&JsonDocument<'_>,object:usize,key:&[u8],nonempty:bool)->bool { get(doc,object,key).is_none_or(|v|valid_string(doc,Some(v),nonempty)) }
fn valid_call_result(doc:&JsonDocument<'_>,object:usize)->bool {
    if doc.token(object).is_none_or(|t|t.kind!=JsonType::Object)||!valid_content(doc,get(doc,object,b"content")){return false;}
    let bool_field=get(doc,object,b"isError").is_none_or(|v|doc.bool(v).is_some());
    let structured=get(doc,object,b"structuredContent").is_none_or(|v|doc.token(v).is_some_and(|t|t.kind==JsonType::Object));
    let kind=get(doc,object,b"resultType").is_none_or(|v|doc.string_equals(v,b"complete"));
    bool_field&&structured&&kind&&optional_object(doc,object,b"_meta")
}
fn valid_task_meta(doc:&JsonDocument<'_>,object:usize,allow_poll:bool)->bool {
    let Some(id)=get(doc,object,b"taskId") else{return false};
    if !valid_string(doc,Some(id),true)||doc.decoded_string(id).is_none_or(|s|s.len()>TASK_ID_LIMIT){return false;}
    let status=[b"working".as_slice(),b"completed",b"failed",b"cancelled"].iter().any(|s|string_is(doc,object,b"status",s));
    let Some(ttl)=get(doc,object,b"ttlMs").and_then(|v|doc.uint(v)) else{return false};
    if !status||!valid_timestamp(doc,object,b"createdAt")||!valid_timestamp(doc,object,b"lastUpdatedAt")||ttl==0||ttl>TASK_TTL_LIMIT{return false;}
    if let Some(p)=get(doc,object,b"pollIntervalMs") { let Some(n)=doc.uint(p) else{return false}; if !allow_poll||n==0||n>TASK_TTL_LIMIT{return false;} }
    if get(doc,object,b"statusMessage").is_some_and(|v|doc.token(v).is_none_or(|t|t.kind!=JsonType::String)) {return false;}
    optional_object(doc,object,b"_meta")
}
fn valid_terminal(doc:&JsonDocument<'_>,object:usize)->bool {
    let result=get(doc,object,b"result"); let error=get(doc,object,b"error");
    if string_is(doc,object,b"status",b"completed") { result.is_some()&&error.is_none()&&result.is_some_and(|v|valid_call_result(doc,v)) }
    else if string_is(doc,object,b"status",b"failed") { result.is_none()&&error.is_some_and(|v|valid_error(doc,v)) }
    else { result.is_none()&&error.is_none() }
}
fn valid_create_task(doc:&JsonDocument<'_>,object:usize)->bool { valid_task_meta(doc,object,true)&&get(doc,object,b"result").is_none()&&get(doc,object,b"error").is_none() }
fn valid_get_task(doc:&JsonDocument<'_>,object:usize)->bool { valid_task_meta(doc,object,false)&&valid_terminal(doc,object) }
fn valid_cancel_ack(doc:&JsonDocument<'_>,root:usize)->bool {
    if !string_is(doc,root,b"resultType",b"complete")||![b"taskId".as_slice(),b"status",b"createdAt",b"lastUpdatedAt",b"ttlMs",b"result",b"error"].iter().all(|k|get(doc,root,k).is_none())||!optional_object(doc,root,b"_meta"){return false;}
    let Some(token)=doc.token(root) else{return false}; let mut key=token.first_child;
    while let Some(k)=key {let Some(value)=doc.token(k).and_then(|t|t.next_sibling) else{return false}; if !doc.string_equals(k,b"resultType")&&!doc.string_equals(k,b"_meta"){return false;} key=doc.token(value).and_then(|t|t.next_sibling);}
    true
}

pub fn legacy_progress(raw_token:&[u8], progress:u32, prior:&mut i64, message:&str, out:&mut JsonBuffer)->Result<(),ProgressError> {
    if raw_token.is_empty()||raw_token.len()>PROGRESS_TOKEN_LIMIT{return Err(ProgressError::InvalidToken);}
    if progress>1000||i64::from(progress)<=*prior{return Err(ProgressError::InvalidProgress);}
    if message.len()>PROGRESS_MESSAGE_LIMIT{return Err(ProgressError::InvalidMessage);}
    let token_doc=JsonDocument::parse(raw_token).map_err(|_|ProgressError::InvalidToken)?;
    let token_type=token_doc.token(token_doc.root()).map(|t|t.kind);
    let scalar=matches!(token_type,Some(JsonType::String)) || (token_type==Some(JsonType::Number)&&token_doc.raw_token(token_doc.root()).is_some_and(|r|!r.iter().any(|b|matches!(b,b'.'|b'e'|b'E'))));
    if !scalar{return Err(ProgressError::InvalidToken);}
    let mut payload=JsonBuffer::new(128,MAX_BYTES).map_err(|_|ProgressError::Buffer)?;
    payload.literal("{\"jsonrpc\":\"2.0\",\"method\":\"notifications/progress\",\"params\":{\"progressToken\":").map_err(|_|ProgressError::Buffer)?;
    payload.append(raw_token).map_err(|_|ProgressError::Buffer)?;
    payload.literal(",\"progress\":").map_err(|_|ProgressError::Buffer)?;
    append_u32(&mut payload,progress).map_err(|_|ProgressError::Buffer)?;
    payload.literal(",\"total\":1000,\"message\":").map_err(|_|ProgressError::Buffer)?;
    payload.quoted(message.as_bytes()).map_err(|_|ProgressError::Buffer)?; payload.literal("}}").map_err(|_|ProgressError::Buffer)?;
    out.append(payload.as_bytes()).map_err(|_|ProgressError::Buffer)?; *prior=i64::from(progress); Ok(())
}

fn append_u32(out:&mut JsonBuffer,value:u32)->Result<(),vexel_json::JsonBufferError>{
    let mut digits=[0u8;10];let mut n=value;let mut at=digits.len();
    loop{at-=1;digits[at]=b'0'+(n%10) as u8;n/=10;if n==0{break;}}
    out.append(&digits[at..])
}
