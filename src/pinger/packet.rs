use std::io::{Error, ErrorKind};

#[repr(C)]
struct IcmpPacket {
    r#type: u8,
    code: u8,
    checksum: u16,
    identifier: u16,
    sequence: u16,
}

#[derive(Debug)]
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
    pub fn identifier(&self) -> u16 {
        match self {
            IcmpMessage::Echo { identifier, .. } => *identifier,
            IcmpMessage::EchoReply { identifier, .. } => *identifier,
        }
    }

    pub fn sequence(&self) -> u16 {
        match self {
            IcmpMessage::Echo { sequence, .. } => *sequence,
            IcmpMessage::EchoReply { sequence, .. } => *sequence,
        }
    }

    pub fn payload(&self) -> &[u8] {
        match self {
            IcmpMessage::Echo { payload, .. } => payload,
            IcmpMessage::EchoReply { payload, .. } => payload,
        }
    }

    pub fn as_v4_bytes(&self) -> Box<[u8]> {
        let r#type = match self {
            IcmpMessage::Echo { .. } => 8,
            IcmpMessage::EchoReply { .. } => 0,
        };
        let mut buffer = self.as_bytes(r#type);
        let packet = unsafe { &mut *(buffer.as_mut_ptr() as *mut IcmpPacket) };
        let mut checksum = buffer.chunks(2).fold(0, |acc, bytes| {
            acc + if bytes.len() == 1 {
                (bytes[0] as u32) << 8
            } else {
                u16::from_be_bytes(bytes.try_into().unwrap()) as u32
            }
        });

        while checksum >> 16 != 0 {
            checksum = (checksum >> 16) + (checksum & 0xFFFF);
        }

        packet.checksum = (!checksum as u16).to_be();

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
        packet.identifier = identifier.to_be();
        packet.sequence = sequence.to_be();

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
            8 | 128 => Ok(IcmpMessage::Echo {
                identifier: u16::from_be(packet.identifier),
                sequence: u16::from_be(packet.sequence),
                payload,
            }),
            0 | 129 => Ok(IcmpMessage::EchoReply {
                identifier: u16::from_be(packet.identifier),
                sequence: u16::from_be(packet.sequence),
                payload,
            }),
            _ => Err(Error::from(ErrorKind::UnexpectedEof)),
        }
    }
}
