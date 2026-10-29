use std::io::{Error, ErrorKind};

#[repr(C)]
struct IcmpPacket {
    r#type: u8,
    code: u8,
    checksum: u16,
    identifier: u16,
    sequence: u16,
}

pub enum IcmpMessage {
    Echo {
        identifier: u16,
        sequence: u16,
        payload: Box<[u8]>,
    },
    EchoReply {
        identifier: u16,
        sequence: u16,
        payload: Box<[u8]>,
    },
}

impl IcmpMessage {
    pub fn as_v4_bytes(&self) -> Box<[u8]> {
        let r#type = match self {
            IcmpMessage::Echo { .. } => 0,
            IcmpMessage::EchoReply { .. } => 8,
        };
        let mut buffer = self.as_bytes(r#type);
        let packet = unsafe { &mut *(buffer.as_mut_ptr() as *mut IcmpPacket) };
        let mut checksum = buffer.chunks(2).fold(0, |acc, bytes| {
            acc + if bytes.len() == 1 {
                (buffer[0] as u32) << 8
            } else {
                u16::from_be_bytes(bytes.try_into().unwrap()) as u32
            }
        });

        while checksum >> 16 != 0 {
            checksum = (checksum >> 16) + (checksum & 0xFFFF);
        }

        packet.checksum = !checksum as u16;

        buffer
    }

    pub fn as_v6_bytes(&self) -> Box<[u8]> {
        let r#type = match self {
            IcmpMessage::Echo { .. } => 128,
            IcmpMessage::EchoReply { .. } => 129,
        };

        self.as_bytes(r#type)
    }

    fn as_bytes(&self, r#type: u8) -> Box<[u8]> {
        let (identifier, sequence, payload) = match self {
            IcmpMessage::Echo {
                identifier,
                sequence,
                payload,
            }
            | IcmpMessage::EchoReply {
                identifier,
                sequence,
                payload,
            } => (identifier, sequence, payload),
        };
        let mut buffer = unsafe {
            Box::<[u8]>::new_uninit_slice(payload.len() + size_of::<IcmpPacket>()).assume_init()
        };
        let packet = unsafe { &mut *(buffer.as_mut_ptr() as *mut IcmpPacket) };

        packet.r#type = r#type;
        packet.code = 0;
        packet.checksum = 0;
        packet.identifier = *identifier;
        packet.sequence = *sequence;

        buffer[size_of::<IcmpPacket>()..].copy_from_slice(&payload);
        buffer
    }
}

impl TryFrom<&[u8]> for IcmpMessage {
    type Error = Error;

    fn try_from(buffer: &[u8]) -> Result<Self, Self::Error> {
        if buffer.len() < size_of::<IcmpPacket>() {
            return Err(Error::from(ErrorKind::UnexpectedEof));
        }

        let packet = unsafe { &*(buffer.as_ptr() as *const IcmpPacket) };

        if packet.code != 0 {
            return Err(Error::from(ErrorKind::InvalidData));
        }

        let mut payload = unsafe {
            Box::<[u8]>::new_uninit_slice(buffer.len() - size_of::<IcmpPacket>()).assume_init()
        };

        payload.copy_from_slice(&buffer[size_of::<IcmpPacket>()..]);

        match packet.r#type {
            0 | 128 => Ok(IcmpMessage::Echo {
                identifier: packet.identifier,
                sequence: packet.sequence,
                payload,
            }),
            8 | 129 => Ok(IcmpMessage::EchoReply {
                identifier: packet.identifier,
                sequence: packet.sequence,
                payload,
            }),
            _ => Err(Error::from(ErrorKind::UnexpectedEof)),
        }
    }
}
