use crate::bitcoin::address::AddressPayload;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum AddressFilter {
    Types(AddressTypes),
    Text(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AddressTypes {
    All,
    None,
    P2PKH,
    P2SH,
    WitnessV0,
    P2TR,
}

impl AddressFilter {
    pub(crate) fn classify(prefix: String, testnet: bool) -> Self {
        let (prefix_testnet, types) = match prefix.as_str() {
            "" => return Self::Types(AddressTypes::All),
            "1" => (false, AddressTypes::P2PKH),
            "3" => (false, AddressTypes::P2SH),
            "bc1q" => (false, AddressTypes::WitnessV0),
            "bc1p" => (false, AddressTypes::P2TR),
            "2" => (true, AddressTypes::P2SH),
            "tb1q" => (true, AddressTypes::WitnessV0),
            "tb1p" => (true, AddressTypes::P2TR),
            _ => return Self::Text(prefix),
        };
        Self::Types(if prefix_testnet == testnet {
            types
        } else {
            AddressTypes::None
        })
    }
}

impl AddressTypes {
    pub(crate) fn matches(self, address: &AddressPayload) -> bool {
        match self {
            Self::All => true,
            Self::None => false,
            Self::P2PKH => matches!(address, AddressPayload::P2PKH(_)),
            Self::P2SH => matches!(address, AddressPayload::P2SH(_)),
            Self::WitnessV0 => matches!(
                address,
                AddressPayload::P2WPKH(_) | AddressPayload::P2WSH(_)
            ),
            Self::P2TR => matches!(address, AddressPayload::P2TR(_)),
        }
    }
}
