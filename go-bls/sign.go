// Copyright (C) 2018 Authors
// distributed under Apache 2.0 license

package main

import (
	"math/big"
	. "bls/curves"
    "fmt"
    "io"
    "os"
	//"testing"
	"crypto/rand"
	"flag"
	"io/ioutil" 
	"encoding/json"
	"encoding/hex"

//	b64 "encoding/base64"
//
	//"github.com/stretchr/testify/assert"
	"golang.org/x/crypto/sha3"
)

func KeyGen(curve CurveSystem) (*big.Int, Point, Point, error) {
	//Create private key
	sk, err := rand.Int(rand.Reader, curve.GetG1Order())

	//Check for error
	if err != nil {
		return nil, nil, nil, err
	}

	//Compute public key
	W, X := ComputePublicKey(curve, sk)

	return sk, W, X, nil
}

type Account struct {
	Sk string
    Pkx1 string
    Pkx2 string
    Pky1 string
    Pky2 string
}

func ComputePublicKey(curve CurveSystem, sk *big.Int) (Point, Point) {
	W := curve.GetG1().Mul(sk)
	X := curve.GetG2().Mul(sk)
	return W, X
}


func Sign(curve CurveSystem, x *big.Int, msg []byte) (Point, Point) {
	//Prepend public key to message
	m := msg

	//Hash message to element in G1
	h := hashToG1(curve, m)

	//Compute signature
	sigma := h.Mul(x)

	return h, sigma
}

func hashToG1(curve CurveSystem, message []byte) Point {
    hash := sha3.NewLegacyKeccak256()
    hash.Write(message)
    var buf []byte
    buf = hash.Sum(buf)
	return curve.GetG1().Mul(new(big.Int).SetBytes(buf))
}

func Gen(curve CurveSystem, filename string) {
	
	//g2 := curve.GetG2()
	sk, _, pk, _ := KeyGen(curve)
	fmt.Println("sk,", sk)
	fmt.Println("pk,", pk.ToAffineCoords())

	pkc := pk.ToAffineCoords()
	data := fmt.Sprintf("{\n\"sk\": \"%v\",\n\"pkx1\": \"%v\",\n\"pkx2\": \"%v\",\n\"pky1\": \"%v\",\n\"pky2\": \"%v\"\n}", sk, pkc[0], pkc[1], pkc[2], pkc[3]) 

	

	file, _ := os.Create(filename)

    defer file.Close()

    _, _ = io.WriteString(file, data)
}

func SignBLS(curve CurveSystem, keyfilename string, msg string) {
	keyfile, _ := os.Open(keyfilename)
	defer keyfile.Close()

	key, _ := ioutil.ReadAll(keyfile)
	msg_bytes, _ := hex.DecodeString(msg)
	var account Account
	json.Unmarshal(key, &account)
	

	key_int := new(big.Int)
	key_int.SetString(account.Sk, 10)
	_, signature := Sign(curve, key_int, msg_bytes)
	fmt.Println(signature.ToAffineCoords()[0])
	fmt.Println(signature.ToAffineCoords()[1])

	
//	g2 := curve.GetG2()
//	pk := g2.Mul(key_int)
//	p1, _ := curve.Pair(signature, g2)
//	p2, _ := curve.Pair(h, pk)

}

func StrToInt(str string) *big.Int {
	x := new(big.Int)
	x.SetString(str, 10)
	return x
}

func main() {
	curve := CurveSystem(Altbn128)
	//idPtr := flag.Int("nid", 1, "node id")
	msgPtr := flag.String("msg", "", "msg to sign")
	keyPtr := flag.String("key", "", "key file")
	


    flag.Parse()
    //filename := fmt.Sprintf("keyfile/node%d", *idPtr)

    //Gen(curve, filename)
  	SignBLS(curve, *keyPtr, *msgPtr)


	

	/*str := "deadbeef"
	msg, _ := hex.DecodeString(str)
	_, sig := Sign(curve, sk, g2pk, msg)
	fmt.Println("sig,", sig.ToAffineCoords())*/
}

/*
func TestBLS(t *testing.T) {
	curve := CurveSystem(Altbn128)
	g2 := curve.GetG2()
	sk, _, g2pk, err := KeyGen(curve)
	if err != nil {
		t.Error("incorrectly key gen" + err.Error())
	}
	fmt.Println("sk", sk)
	//fmt.Println(g1pk.ToAffineCoords())
	fmt.Println(g2pk.ToAffineCoords())

	str := "deadbeef"
	msg, err1 := hex.DecodeString(str)
	if err1 != nil {
		t.Error("incorrectly decode string" + err1.Error())
	}
	h, sig := Sign(curve, sk, g2pk, msg)
	//sig := msgPt.Mul(sk1)
	//fmt.Println(sig.ToAffineCoords())
	p1, _ := curve.Pair(sig, g2)
	p2, _ := curve.Pair(h, g2pk)
	//p3, _ := curve.Pair(sig, pk1)
	assert.True(t, p1.Equals(p2), "paring check failed")

}

func TestAggBLS(t *testing.T) {
	curve := CurveSystem(Altbn128)
	g2 := curve.GetG2()
	sk1, _, g2pk1, err1 := KeyGen(curve)
	sk2, _, g2pk2, err2 := KeyGen(curve)
	if err1 != nil || err2 != nil{
		t.Error("incorrectly key gen")
	}
	aggpk, _ := g2pk1.Add(g2pk2)
	fmt.Println(aggpk.ToAffineCoords())

	str := "deadbeef"
	msg, err3 := hex.DecodeString(str)
	if err3 != nil {
		t.Error("incorrectly decode string" + err3.Error())
	}
	h1, sig1 := Sign(curve, sk1, g2pk1, msg)
	h2, sig2 := Sign(curve, sk2, g2pk2, msg)
	aggsig, _ := sig1.Add(sig2)
	//sig := msgPt.Mul(sk1)
	fmt.Println(aggsig.ToAffineCoords())
	p1, _ := curve.Pair(aggsig, g2)
	p2, _ := curve.Pair(h1, aggpk)
	//p3, _ := curve.Pair(sig, pk1)
	assert.True(t, p1.Equals(p2), "paring check failed")
	assert.True(t, h1.Equals(h2), "paring check failed")
	fmt.Println(curve.GetG2().ToAffineCoords())

}
*/


