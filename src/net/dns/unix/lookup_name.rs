fn is_valid_hostname(host: &[u8]) -> bool {
    todo!()
}



fn name_from_null(name: &[u8],  family: i32,  flags: i32) -> ([IpAddr; 2], usize)
{
	int cnt = 0;
	if (name) return 0;

    
	if (flags & AI_PASSIVE) {
		if (family != AF_INET6)
			buf[cnt++] = (struct address){ .family = AF_INET };
		if (family != AF_INET)
			buf[cnt++] = (struct address){ .family = AF_INET6 };
	} else {
		if (family != AF_INET6)
			buf[cnt++] = (struct address){ .family = AF_INET, .addr = { 127,0,0,1 } };
		if (family != AF_INET)
			buf[cnt++] = (struct address){ .family = AF_INET6, .addr = { [15] = 1 } };
	}
	return cnt;
}