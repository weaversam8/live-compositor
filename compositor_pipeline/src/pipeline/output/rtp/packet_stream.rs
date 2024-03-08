use std::collections::VecDeque;

use crossbeam_channel::Receiver;

use crate::pipeline::structs::EncoderOutputEvent;

use super::payloader::{Payloader, PayloadingError};

pub(super) struct PacketStream {
    packets_receiver: Receiver<EncoderOutputEvent>,
    state: VecDeque<bytes::Bytes>,
    payloader: Payloader,
    mtu: usize,
}

impl PacketStream {
    pub(super) fn new(
        packets_receiver: Receiver<EncoderOutputEvent>,
        payloader: Payloader,
        mtu: usize,
    ) -> Self {
        Self {
            packets_receiver,
            payloader,
            mtu,
            state: VecDeque::new(),
        }
    }

    fn next_new_packet(&mut self) -> Option<Result<bytes::Bytes, PayloadingError>> {
        let Ok(packet) = self.packets_receiver.recv() else {
            if let Some(packet) = self.payloader.audio_eos() {
                return Some(packet);
            }
            if let Some(packet) = self.payloader.video_eos() {
                return Some(packet);
            }
            return None;
        };

        let encoded_chunk = match packet {
            EncoderOutputEvent::Data(packet) => packet,
            EncoderOutputEvent::AudioEOS => {
                return Some(
                    self.payloader
                        .audio_eos()
                        .unwrap_or(Err(PayloadingError::AudioEOSAlreadySent)),
                )
            }
            EncoderOutputEvent::VideoEOS => {
                return Some(
                    self.payloader
                        .video_eos()
                        .unwrap_or(Err(PayloadingError::VideoEOSAlreadySent)),
                )
            }
        };

        let rtp_packets = match self.payloader.payload(self.mtu, encoded_chunk) {
            Ok(packets) => packets,
            Err(err) => return Some(Err(err)),
        };

        // I'm assuming here that payload will never return empty list
        self.state = rtp_packets;
        self.state.pop_front().map(Ok)
    }
}

impl Iterator for PacketStream {
    type Item = Result<bytes::Bytes, PayloadingError>;

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.state {
            chunks if chunks.is_empty() => self.next_new_packet(),
            chunks => chunks.pop_front().map(Ok),
        }
    }
}
