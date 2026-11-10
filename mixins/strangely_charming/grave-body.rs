fn hook(mut body: BodyFlags, guest: &Guest) -> BodyFlags {
    body.invisible |= crate::sc_grave_veil::guest_body(guest.uuid, guest.conn);
    body
}
