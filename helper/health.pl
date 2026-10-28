#!/usr/bin/perl
# How loaded this machine is, for Shepherdr on the main machine (health.ts): cores, 1-minute load, memory, how full
# home and /tmp are, a GPU's use. One JSON line. Linux and macOS. Put here by Shepherdr.
use strict;
use warnings;
use JSON::PP;

my $os = `uname -s`; chomp $os;
my ($cores, $load, $total, $used) = (1, 0, 0, 0);
if ($os eq 'Darwin') {
  $cores = `sysctl -n hw.ncpu` + 0 || 1;
  ($load) = `sysctl -n vm.loadavg` =~ /([\d.]+)/;
  $total = `sysctl -n hw.memsize` + 0;
  my $vm = `vm_stat`;
  my ($page) = $vm =~ /page size of (\d+)/;
  my %p = map { $_ => ($vm =~ /^Pages \Q$_\E:\s+(\d+)/m ? $1 : 0) } ('free', 'inactive', 'speculative', 'purgeable');
  $used = $total - ($p{free} + $p{inactive} + $p{speculative} + $p{purgeable}) * ($page || 4096);
} else {
  my $read = sub { open(my $f, '<', $_[0]) or return ''; local $/; my $t = <$f>; close $f; $t };
  $cores = (() = $read->('/proc/cpuinfo') =~ /^processor\s*:/mg) || 1;
  ($load) = split ' ', $read->('/proc/loadavg');
  my $m = $read->('/proc/meminfo');
  my ($t) = $m =~ /^MemTotal:\s+(\d+)/m;
  my ($a) = $m =~ /^MemAvailable:\s+(\d+)/m;
  ($a) = $m =~ /^MemFree:\s+(\d+)/m unless defined $a;
  $total = ($t // 0) * 1024;
  $used = $total - ($a // 0) * 1024;
}
my (@disks, %seen);
for my $p ($ENV{HOME}, '/tmp') {
  my @l = `df -Pk '$p' 2>/dev/null`;
  next unless @l > 1;
  my @f = split ' ', $l[-1];
  my ($size, $u, $avail, $dev) = ($f[1], $f[2], $f[3], $f[0]);
  next if $seen{$dev}++;
  my $pct = ($u + $avail) ? int(100 * $u / ($u + $avail) + 0.5) : 0;
  push @disks, { path => $p, pct => $pct, freeGb => $avail / 1048576, dev => $dev };
}
my @gpus;
for my $l (`nvidia-smi --query-gpu=name,utilization.gpu,memory.used,memory.total --format=csv,noheader,nounits 2>/dev/null`) {
  my ($name, $util, $mu, $mt) = map { s/^\s+|\s+$//gr } split /,/, $l;
  next unless defined $mt;
  push @gpus, { name => $name, util => $util + 0, memPct => $mt ? int(100 * $mu / $mt + 0.5) : 0 };
}
print JSON::PP->new->canonical->encode({ cores => $cores + 0, load => ($load // 0) + 0, memTotal => $total + 0, memUsed => ($used > 0 ? $used : 0) + 0, disks => \@disks, gpus => \@gpus }), "\n";
