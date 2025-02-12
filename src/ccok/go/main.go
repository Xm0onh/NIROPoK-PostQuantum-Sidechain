package main

import (
	"crypto/rand"
	"fmt"
	mrand "math/rand"
	"time"

	"golang.org/x/crypto/sha3"
)

func main() {
	// Seed math/rand for random weight generation
	mrand.Seed(time.Now().UnixNano())

	// Number of participants
	numParticipants := 10

	// Create slices for participants and signers
	participants := make([]Participant, numParticipants)
	signers := make([]*SchnorrSigner, numParticipants)
	var totalWeight uint64 = 0

	// Generate participants with Schnorr signers and random weights (between 10 and 100)
	for i := 0; i < numParticipants; i++ {
		// Generate Schnorr signer using compactcert's function
		signer, err := GenerateSchnorrSigner(rand.Reader)
		if err != nil {
			fmt.Printf("Error generating Schnorr signer for participant %d: %v\n", i, err)
			return
		}
		signers[i] = signer

		// Assign a random weight in the range [10, 100]
		weight := uint64(10 + mrand.Intn(91))
		totalWeight += weight

		participants[i] = Participant{
			PK:     signer.Public(),
			Weight: weight,
		}
	}

	// Define a message for signing
	msg := []byte("Threshold signature test message")

	// Set proven threshold as half of total weight
	provenWeight := totalWeight / 2
	fmt.Printf("Total weight: %d, Proven threshold (ProvenWeight): %d\n", totalWeight, provenWeight)

	// Build party tree from participants
	partsBytes, err := Participants(participants).Bytes()
	if err != nil {
		fmt.Printf("Error serializing participants: %v\n", err)
		return
	}
	partyTree := NewMerkleTree().Build(partsBytes)

	// Create Params object
	params := Params{
		Msg:          msg,
		ProvenWeight: provenWeight,
		SecKQ:        128, // Setting a proper security parameter
	}

	// Create the Builder
	builder := NewBuilder(params, participants, partyTree)

	// Each participant signs the message using their Schnorr signer
	for i, signer := range signers {
		// Create a new hash instance for each signature
		h := sha3.New256()
		sig, err := signer.Sign(msg, h)
		if err != nil {
			fmt.Printf("Error signing for participant %d: %v\n", i, err)
			return
		}
		// Add the signature to the builder
		err = builder.AddSignature(i, sig)
		if err != nil {
			fmt.Printf("Error adding signature for participant %d: %v\n", i, err)
			return
		}
		fmt.Printf("Participant %d signed (weight %d)\n", i, participants[i].Weight)
	}

	// Build the certificate
	cert, err := builder.Build()
	if err != nil {
		fmt.Printf("Error building certificate: %v\n", err)
		return
	}
	fmt.Printf("Certificate built successfully. Signed weight: %d\n", cert.SignedWeight)

	// Create verifier using the same parameters and party tree root
	verifier := NewVerifier(params, partyTree.Root())
	err = verifier.Verify(cert)
	if err != nil {
		fmt.Printf("Certificate verification failed: %v\n", err)
	} else {
		fmt.Println("Certificate verified successfully!")
	}
}
